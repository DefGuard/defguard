use std::collections::HashSet;

use ldap3::{Mod, SearchEntry};
use sqlx::{Error as SqlxError, PgExecutor};

use super::{error::LdapError, LDAPConfig};
use crate::{
    db::{Id, Settings, User},
    handlers::user::check_username,
    hashset,
};

pub(crate) enum UserObjectClass {
    SambaSamAccount,
    InetOrgPerson,
    SimpleSecurityObject,
    User,
}

impl<'a> From<&'a UserObjectClass> for &'static str {
    fn from(obj_class: &'a UserObjectClass) -> &'static str {
        match obj_class {
            UserObjectClass::SambaSamAccount => "sambaSamAccount",
            UserObjectClass::InetOrgPerson => "inetOrgPerson",
            UserObjectClass::SimpleSecurityObject => "simpleSecurityObject",
            UserObjectClass::User => "user",
        }
    }
}

impl From<UserObjectClass> for &'static str {
    fn from(obj_class: UserObjectClass) -> &'static str {
        (&obj_class).into()
    }
}

impl From<UserObjectClass> for String {
    fn from(obj_class: UserObjectClass) -> String {
        let str: &str = obj_class.into();
        str.to_string()
    }
}

impl PartialEq<&str> for UserObjectClass {
    fn eq(&self, other: &&str) -> bool {
        let str: &str = self.into();
        str == *other
    }
}

impl PartialEq<UserObjectClass> for &str {
    fn eq(&self, other: &UserObjectClass) -> bool {
        other == self
    }
}

impl User {
    pub fn from_searchentry(
        entry: &SearchEntry,
        username: &str,
        password: Option<&str>,
    ) -> Result<Self, LdapError> {
        let mut user = Self::new(
            username.into(),
            password,
            get_value_or_error(entry, "sn")?,
            get_value_or_error(entry, "givenName")?,
            get_value_or_error(entry, "mail")?,
            get_value(entry, "mobile"),
        );
        user.from_ldap = true;
        if let Some(rdn) = extract_rdn_value(&entry.dn) {
            user.ldap_rdn = Some(rdn);
        } else {
            return Err(LdapError::InvalidDN(entry.dn.clone()));
        }
        if let Some(dn_path) = extract_dn_path(&entry.dn) {
            user.ldap_user_path = Some(dn_path);
        } else {
            return Err(LdapError::InvalidDN(entry.dn.clone()));
        }
        // Print the warning only if everything else checks out
        if check_username(username).is_err() {
            warn!(
                "LDAP User \"{}\" has username that cannot be used in Defguard, \
                change the LDAP username attribute or change the username in LDAP to a valid one",
                username
            );
            return Err(LdapError::InvalidUsername(username.to_string()));
        }
        Ok(user)
    }
}

impl<I> User<I> {
    pub(crate) fn update_from_ldap_user(&mut self, ldap_user: &User, config: &LDAPConfig) {
        self.last_name = ldap_user.last_name.clone();
        self.first_name = ldap_user.first_name.clone();
        self.email = ldap_user.email.clone();
        self.phone = ldap_user.phone.clone();
        // It should be ok to update the username if we are not using it in the DN (not as RDN)
        if !config.using_username_as_rdn() {
            self.username = ldap_user.username.clone();
        } else {
            debug!(
                "Not updating username {} from LDAP because it is used as RDN",
                self.username
            );
        }
    }

    #[must_use]
    pub fn as_ldap_mod<'a>(&'a self, config: &'a LDAPConfig) -> Vec<Mod<&'a str>> {
        let obj_classes = config.get_all_user_obj_classes();
        let mut changes = vec![];
        if obj_classes.contains(&UserObjectClass::InetOrgPerson.into())
            || obj_classes.contains(&UserObjectClass::User.into())
        {
            changes.extend_from_slice(&[
                Mod::Replace("sn", hashset![self.last_name.as_str()]),
                Mod::Replace("givenName", hashset![self.first_name.as_str()]),
                Mod::Replace("mail", hashset![self.email.as_str()]),
            ]);

            // Allow renaming the user if the CN is not a part of the RDN
            if config.get_rdn_attr() != "cn" {
                changes.push(Mod::Replace("cn", hashset![self.username.as_str()]));
            }

            if config.ldap_username_attr != "uid" && config.ldap_user_rdn_attr != Some("uid".into())
            {
                changes.push(Mod::Replace("uid", hashset![self.username.as_str()]));
            }

            if let Some(phone) = &self.phone {
                if phone.is_empty() {
                    changes.push(Mod::Replace("mobile", HashSet::new()));
                } else {
                    changes.push(Mod::Replace("mobile", hashset![phone.as_str()]));
                }
            }
        } else {
            warn!(
                "No user object class found for user {}, can't generate mods",
                self.username
            );
        }

        if config.ldap_uses_ad && config.get_rdn_attr() != "sAMAccountName" {
            changes.push(Mod::Replace(
                "sAMAccountName",
                hashset![self.username.as_str()],
            ));
        }

        let username_attr = config.ldap_username_attr.as_str();
        // add anything the user provided, if we haven't already added it AND it's not the same as the RDN
        if username_attr != "sAMAccountName"
            && username_attr != "cn"
            && Some(username_attr.into()) != config.ldap_user_rdn_attr
        {
            changes.push(Mod::Replace(
                username_attr,
                hashset![self.username.as_str()],
            ));
        }

        changes
    }

    // check if key is already in attrs, if not return false
    fn in_attrs<'a>(attrs: &'a Vec<(&'a str, HashSet<&'a str>)>, key: &str) -> bool {
        attrs.iter().any(|(k, _)| *k == key)
    }

    #[must_use]
    pub fn as_ldap_attrs<'a>(
        &'a self,
        ssha_password: &'a str,
        nt_password: &'a str,
        object_classes: HashSet<&'a str>,
        uses_ad: bool,
        username_attr: &'a str,
        rdn_attr: &'a str,
    ) -> Vec<(&'a str, HashSet<&'a str>)> {
        let mut attrs = vec![];
        attrs.push((rdn_attr, hashset![self.ldap_rdn_value()]));
        if object_classes.contains(UserObjectClass::InetOrgPerson.into())
            || object_classes.contains(UserObjectClass::User.into())
        {
            attrs.extend_from_slice(&[
                ("sn", hashset![self.last_name.as_str()]),
                ("givenName", hashset![self.first_name.as_str()]),
                ("mail", hashset![self.email.as_str()]),
            ]);

            if !Self::in_attrs(&attrs, "cn") {
                attrs.push(("cn", hashset![self.username.as_str()]));
            }

            if !Self::in_attrs(&attrs, "uid") {
                attrs.push(("uid", hashset![self.username.as_str()]));
            }

            if let Some(phone) = &self.phone {
                if !phone.is_empty() {
                    attrs.push(("mobile", hashset![phone.as_str()]));
                }
            }
        }
        if object_classes.contains(UserObjectClass::SimpleSecurityObject.into()) {
            // simpleSecurityObject
            attrs.push(("userPassword", hashset![ssha_password]));
        }
        if object_classes.contains(UserObjectClass::SambaSamAccount.into()) {
            // sambaSamAccount
            attrs.push(("sambaSID", hashset!["0"]));
            attrs.push(("sambaNTPassword", hashset![nt_password]));
        }
        if uses_ad {
            attrs.push(("sAMAccountName", hashset![self.username.as_str()]));
        }

        // Add the username attr and RDN if we haven't already added it
        if !Self::in_attrs(&attrs, username_attr) {
            attrs.push((username_attr, hashset![self.username.as_str()]));
        }

        attrs.push(("objectClass", object_classes));

        debug!("Generated LDAP attributes: {:?}", attrs);

        attrs
    }

    /// Updates the LDAP RDN value of the user in Defguard, if Defguard uses the usernames as RDN.
    pub(crate) async fn maybe_update_rdn(&mut self) {
        debug!("Updating RDN for user {} in Defguard", self.username);
        let settings = Settings::get_current_settings();
        if settings.ldap_using_username_as_rdn() {
            debug!("The user's username is being used as the RDN, setting it to username");
            self.ldap_rdn = Some(self.username.clone());
        } else {
            debug!("The user's username is NOT being used as the RDN, skipping update");
        }
    }
}

impl User<Id> {
    /// User is syncable with LDAP if:
    /// - he is in a group that is allowed to be synced or no such groups are configured
    /// - he is active (not disabled)
    /// - he is enrolled
    pub(crate) async fn ldap_sync_allowed<'e, E>(&self, executor: E) -> Result<bool, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let sync_groups = Settings::get_current_settings().ldap_sync_groups;
        let my_groups = self.member_of(executor).await?;
        Ok(
            (sync_groups.is_empty() || my_groups.iter().any(|g| sync_groups.contains(&g.name)))
                && self.is_active
                && self.is_enrolled(),
        )
    }

    pub(super) async fn get_without_ldap_path<'e, E>(executor: E) -> Result<Vec<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        sqlx::query_as!(
            Self,
            "
            SELECT id, username, password_hash, last_name, first_name, email, phone, \
            mfa_enabled, totp_enabled, email_mfa_enabled, totp_secret, email_mfa_secret, \
            mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub, \
            from_ldap, ldap_pass_randomized, ldap_rdn, ldap_user_path \
            FROM \"user\" WHERE ldap_user_path IS NULL
            ",
        )
        .fetch_all(executor)
        .await
    }
}

fn get_value_or_error(entry: &SearchEntry, key: &str) -> Result<String, LdapError> {
    match entry.attrs.get(key) {
        Some(values) if !values.is_empty() => Ok(values[0].clone()),
        _ => Err(LdapError::MissingAttribute(key.to_string())),
    }
}

fn get_value(entry: &SearchEntry, key: &str) -> Option<String> {
    match entry.attrs.get(key) {
        Some(values) if !values.is_empty() => Some(values[0].clone()),
        _ => None,
    }
}

/// Get first value from distinguished name, for example: cn=<value>,...
#[must_use]
pub(crate) fn extract_rdn_value(dn: &str) -> Option<String> {
    if let (Some(eq_index), Some(comma_index)) = (dn.find('='), dn.find(',')) {
        dn.get((eq_index + 1)..comma_index).map(ToString::to_string)
    } else {
        None
    }
}

/// Extract the remaining part of the distinguished name after the first comma, for example:
/// `cn=user,dc=example,dc=com` should return `dc=example,dc=com`.
#[must_use]
pub(crate) fn extract_dn_path(dn: &str) -> Option<String> {
    if let Some(parts) = dn.split_once(',') {
        let path = parts.1.to_string();
        debug!("Extracted DN path '{}' from DN '{}'", path, dn);
        Some(path)
    } else {
        warn!("Failed to extract DN path from '{}': no comma found", dn);
        None
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use ldap3::SearchEntry;
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

    use super::*;
    use crate::db::{
        models::settings::{initialize_current_settings, update_current_settings, Settings},
        setup_pool, Group, User,
    };

    fn make_test_user(username: &str) -> User {
        User::new(
            username,
            Some("test_password"),
            "last name",
            "first name",
            format!("{username}@example.com").as_str(),
            None,
        )
    }

    #[sqlx::test]
    async fn test_get_empty_user_path(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let _ = initialize_current_settings(&pool).await;
        let user = make_test_user("testuser");
        let user = user.save(&pool).await.unwrap();

        let mut users = User::<Id>::get_without_ldap_path(&pool).await.unwrap();
        let user_found = users.pop().unwrap();
        assert_eq!(user_found.username, user.username);
    }

    #[test]
    fn test_extract_dn_value() {
        assert_eq!(
            extract_rdn_value("cn=testuser,dc=example,dc=com"),
            Some("testuser".to_string())
        );
        assert_eq!(
            extract_rdn_value("cn=Test User,dc=example,dc=com"),
            Some("Test User".to_string())
        );
        assert_eq!(
            extract_rdn_value("cn=user.name+123,dc=example,dc=com"),
            Some("user.name+123".to_string())
        );
        assert_eq!(extract_rdn_value("invalid-dn"), None);
        assert_eq!(extract_rdn_value("cn=onlyvalue"), None);
        assert_eq!(
            extract_rdn_value("cn=,dc=example,dc=com"),
            Some("".to_string())
        );
        assert_eq!(extract_rdn_value(""), None);
    }

    #[test]
    fn test_from_searchentry() {
        // all attributes
        {
            let mut attrs = HashMap::new();
            attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
            attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
            attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);
            attrs.insert("mobile".to_string(), vec!["1234567890".to_string()]);

            let entry = SearchEntry {
                dn: "cn=user1,dc=example,dc=com".to_string(),
                attrs,
                bin_attrs: HashMap::new(),
            };

            let user = User::from_searchentry(&entry, "user1", Some("password123")).unwrap();

            assert_eq!(user.username, "user1");
            assert_eq!(user.last_name, "lastname1");
            assert_eq!(user.first_name, "firstname1");
            assert_eq!(user.email, "user1@example.com");
            assert_eq!(user.phone, Some("1234567890".to_string()));
            assert!(user.from_ldap);
        }

        // without mobile
        {
            let mut attrs = HashMap::new();
            attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
            attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
            attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);

            let entry = SearchEntry {
                dn: "cn=user1,dc=example,dc=com".to_string(),
                attrs,
                bin_attrs: HashMap::new(),
            };

            let user = User::from_searchentry(&entry, "user1", None).unwrap();

            assert_eq!(user.username, "user1");
            assert_eq!(user.last_name, "lastname1");
            assert_eq!(user.first_name, "firstname1");
            assert_eq!(user.email, "user1@example.com");
            assert_eq!(user.phone, None);
            assert!(user.from_ldap);
        }

        // missing givenName attribute
        {
            let mut attrs = HashMap::new();
            attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
            attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);

            let entry = SearchEntry {
                dn: "cn=user1,dc=example,dc=com".to_string(),
                attrs,
                bin_attrs: HashMap::new(),
            };

            let result = User::from_searchentry(&entry, "user1", None);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                LdapError::MissingAttribute(attr) if attr == "givenName"
            ));
        }

        // missing sn attribute
        {
            let mut attrs = HashMap::new();
            attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
            attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);

            let entry = SearchEntry {
                dn: "cn=user1,dc=example,dc=com".to_string(),
                attrs,
                bin_attrs: HashMap::new(),
            };

            let result = User::from_searchentry(&entry, "user1", None);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                LdapError::MissingAttribute(attr) if attr == "sn"
            ));
        }

        // missing mail attribute
        {
            let mut attrs = HashMap::new();
            attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
            attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);

            let entry = SearchEntry {
                dn: "cn=user1,dc=example,dc=com".to_string(),
                attrs,
                bin_attrs: HashMap::new(),
            };

            let result = User::from_searchentry(&entry, "user1", None);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                LdapError::MissingAttribute(attr) if attr == "mail"
            ));
        }

        // empty attribute values
        {
            let mut attrs = HashMap::new();
            attrs.insert("sn".to_string(), vec![]);
            attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
            attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);

            let entry = SearchEntry {
                dn: "cn=user1,dc=example,dc=com".to_string(),
                attrs,
                bin_attrs: HashMap::new(),
            };

            let result = User::from_searchentry(&entry, "user1", None);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                LdapError::MissingAttribute(attr) if attr == "sn"
            ));
        }

        // invalid DN
        {
            let mut attrs = HashMap::new();
            attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
            attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
            attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);

            let entry = SearchEntry {
                dn: "cn=user1".to_string(), // No comma, invalid DN
                attrs,
                bin_attrs: HashMap::new(),
            };

            let result = User::from_searchentry(&entry, "user1", None);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                LdapError::InvalidDN(dn) if dn == "cn=user1"
            ));

            let mut attrs = HashMap::new();
            attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
            attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
            attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);

            let entry = SearchEntry {
                dn: "user1,dc=example,dc=com".to_string(), // No equals sign in RDN
                attrs,
                bin_attrs: HashMap::new(),
            };

            let result = User::from_searchentry(&entry, "user1", None);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                LdapError::InvalidDN(dn) if dn == "user1,dc=example,dc=com"
            ));
        }

        // invalid username
        {
            let mut attrs = HashMap::new();
            attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
            attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
            attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);

            let entry = SearchEntry {
                dn: "cn=user1,dc=example,dc=com".to_string(),
                attrs,
                bin_attrs: HashMap::new(),
            };

            // Test with invalid username (contains special characters)
            let result = User::from_searchentry(&entry, "user@#$%", None);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                LdapError::InvalidUsername(username) if username == "user@#$%"
            ));
        }

        // complex DN
        {
            let mut attrs = HashMap::new();
            attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
            attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
            attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);
            attrs.insert("mobile".to_string(), vec!["1234567890".to_string()]);

            let entry = SearchEntry {
                dn: "uid=user1,ou=People,ou=Department,dc=example,dc=com".to_string(),
                attrs,
                bin_attrs: HashMap::new(),
            };

            let user = User::from_searchentry(&entry, "user1", Some("password123")).unwrap();

            assert_eq!(user.username, "user1");
            assert_eq!(user.last_name, "lastname1");
            assert_eq!(user.first_name, "firstname1");
            assert_eq!(user.email, "user1@example.com");
            assert_eq!(user.phone, Some("1234567890".to_string()));
            assert!(user.from_ldap);
            assert_eq!(user.ldap_rdn, Some("user1".to_string()));
            assert_eq!(
                user.ldap_user_path,
                Some("ou=People,ou=Department,dc=example,dc=com".to_string())
            );
        }

        // with password
        {
            let mut attrs = HashMap::new();
            attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
            attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
            attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);

            let entry = SearchEntry {
                dn: "cn=user1,dc=example,dc=com".to_string(),
                attrs,
                bin_attrs: HashMap::new(),
            };

            let user = User::from_searchentry(&entry, "user1", Some("mypassword")).unwrap();

            assert_eq!(user.username, "user1");
            assert!(user.password_hash.is_some());
            assert!(user.from_ldap);
        }

        // multiple attribute values
        {
            let mut attrs = HashMap::new();
            attrs.insert(
                "sn".to_string(),
                vec!["lastname1".to_string(), "lastname2".to_string()],
            );
            attrs.insert(
                "givenName".to_string(),
                vec!["firstname1".to_string(), "firstname2".to_string()],
            );
            attrs.insert(
                "mail".to_string(),
                vec![
                    "user1@example.com".to_string(),
                    "user1@other.com".to_string(),
                ],
            );
            attrs.insert(
                "mobile".to_string(),
                vec!["1234567890".to_string(), "0987654321".to_string()],
            );

            let entry = SearchEntry {
                dn: "cn=user1,dc=example,dc=com".to_string(),
                attrs,
                bin_attrs: HashMap::new(),
            };

            let user = User::from_searchentry(&entry, "user1", None).unwrap();

            // Should use the first value when multiple values are present
            assert_eq!(user.last_name, "lastname1");
            assert_eq!(user.first_name, "firstname1");
            assert_eq!(user.email, "user1@example.com");
            assert_eq!(user.phone, Some("1234567890".to_string()));
            assert!(user.from_ldap);
        }

        // fields properly set
        {
            let mut attrs = HashMap::new();
            attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
            attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
            attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);

            let entry = SearchEntry {
                dn: "cn=testuser,ou=users,dc=example,dc=com".to_string(),
                attrs,
                bin_attrs: HashMap::new(),
            };

            let user = User::from_searchentry(&entry, "testuser", None).unwrap();

            // Verify LDAP-specific fields are properly set
            assert!(user.from_ldap);
            assert_eq!(user.ldap_rdn, Some("testuser".to_string()));
            assert_eq!(
                user.ldap_user_path,
                Some("ou=users,dc=example,dc=com".to_string())
            );
        }
    }

    #[test]
    fn test_as_ldap_attrs() {
        let user = User::new(
            "testuser".to_string(),
            Some("password123"),
            "Smith".to_string(),
            "John".to_string(),
            "john.smith@example.com".to_string(),
            Some("5551234".to_string()),
        );

        // Basic test with InetOrgPerson
        let attrs = user.as_ldap_attrs(
            "{SSHA}hashedpw",
            "NT_HASH",
            hashset![UserObjectClass::InetOrgPerson.into()],
            false,
            "uid",
            "cn",
        );

        assert!(attrs.contains(&("cn", hashset!["testuser"])));
        assert!(attrs.contains(&("sn", hashset!["Smith"])));
        assert!(attrs.contains(&("givenName", hashset!["John"])));
        assert!(attrs.contains(&("mail", hashset!["john.smith@example.com"])));
        assert!(attrs.contains(&("mobile", hashset!["5551234"])));
        assert!(attrs.contains(&("objectClass", hashset!["inetOrgPerson"])));

        // Test with ActiveDirectory
        let attrs = user.as_ldap_attrs(
            "{SSHA}hashedpw",
            "NT_HASH",
            hashset![UserObjectClass::User.into()],
            true,
            "uid",
            "cn",
        );

        assert!(attrs.contains(&("sAMAccountName", hashset!["testuser"])));

        // Test with SimpleSecurityObject and SambaSamAccount
        let attrs = user.as_ldap_attrs(
            "{SSHA}hashedpw",
            "NT_HASH",
            hashset![
                UserObjectClass::SimpleSecurityObject.into(),
                UserObjectClass::SambaSamAccount.into()
            ],
            false,
            "uid",
            "uid",
        );

        assert!(attrs.contains(&("userPassword", hashset!["{SSHA}hashedpw"])));
        assert!(attrs.contains(&("sambaSID", hashset!["0"])));
        assert!(attrs.contains(&("sambaNTPassword", hashset!["NT_HASH"])));

        // Test with custom RDN attribute
        let attrs = user.as_ldap_attrs(
            "{SSHA}hashedpw",
            "NT_HASH",
            hashset![UserObjectClass::User.into()],
            false,
            "uid",
            "customRDN",
        );

        assert!(attrs.contains(&("customRDN", hashset![user.ldap_rdn_value()])));
        assert!(attrs.contains(&("uid", hashset!["testuser"])));

        // Test with empty phone
        let user_no_phone = User::new(
            "testuser".to_string(),
            Some("password123"),
            "Smith".to_string(),
            "John".to_string(),
            "john.smith@example.com".to_string(),
            Some("".to_string()),
        );

        let attrs = user_no_phone.as_ldap_attrs(
            "{SSHA}hashedpw",
            "NT_HASH",
            hashset![UserObjectClass::InetOrgPerson.into()],
            false,
            "uid",
            "cn",
        );

        assert!(!attrs.iter().any(|(key, _)| *key == "mobile"));
    }

    #[test]
    fn test_as_ldap_mod_inetorgperson() {
        let user = User::new(
            "testuser".to_string(),
            Some("password123"),
            "Smith".to_string(),
            "John".to_string(),
            "john.smith@example.com".to_string(),
            Some("5551234".to_string()),
        );

        let config = LDAPConfig {
            ldap_user_rdn_attr: Some("cn".to_string()),
            ldap_username_attr: "uid".to_string(),
            ..Default::default()
        };

        let mods = user.as_ldap_mod(&config);
        assert!(mods.contains(&Mod::Replace("sn", hashset!["Smith"])));
        assert!(mods.contains(&Mod::Replace("givenName", hashset!["John"])));
        assert!(mods.contains(&Mod::Replace("mail", hashset!["john.smith@example.com"])));
        assert!(mods.contains(&Mod::Replace("mobile", hashset!["5551234"])));
    }

    #[test]
    fn test_as_ldap_mod_with_empty_phone() {
        let user = User::new(
            "testuser".to_string(),
            Some("password123"),
            "Smith".to_string(),
            "John".to_string(),
            "john.smith@example.com".to_string(),
            Some("".to_string()),
        );

        let config = LDAPConfig {
            ldap_user_rdn_attr: Some("cn".to_string()),
            ldap_username_attr: "uid".to_string(),
            ..Default::default()
        };

        let mods = user.as_ldap_mod(&config);

        assert!(mods.contains(&Mod::Replace("sn", hashset!["Smith"])));
        assert!(mods.contains(&Mod::Replace("givenName", hashset!["John"])));
        assert!(mods.contains(&Mod::Replace("mail", hashset!["john.smith@example.com"])));
        assert!(mods.contains(&Mod::Replace("mobile", HashSet::new())));
    }

    #[test]
    fn test_as_ldap_mod_with_active_directory() {
        let user = User::new(
            "testuser".to_string(),
            Some("password123"),
            "Smith".to_string(),
            "John".to_string(),
            "john.smith@example.com".to_string(),
            Some("5551234".to_string()),
        );

        let config = LDAPConfig {
            ldap_user_obj_class: "user".to_string(),
            ldap_user_rdn_attr: Some("cn".to_string()),
            ldap_username_attr: "sAMAccountName".to_string(),
            ldap_uses_ad: true,
            ..Default::default()
        };

        let mods = user.as_ldap_mod(&config);

        assert!(mods.contains(&Mod::Replace("sn", hashset!["Smith"])));
        assert!(mods.contains(&Mod::Replace("givenName", hashset!["John"])));
        assert!(mods.contains(&Mod::Replace("mail", hashset!["john.smith@example.com"])));
        assert!(mods.contains(&Mod::Replace("sAMAccountName", hashset!["testuser"])));
    }

    #[test]
    fn test_as_ldap_mod_with_custom_rdn() {
        let user = User::new(
            "testuser".to_string(),
            Some("password123"),
            "Smith".to_string(),
            "John".to_string(),
            "john.smith@example.com".to_string(),
            Some("5551234".to_string()),
        );

        let config = LDAPConfig {
            ldap_user_rdn_attr: Some("customRDN".to_string()),
            ldap_username_attr: "uid".to_string(),
            ldap_uses_ad: true,
            ..Default::default()
        };

        let mods = user.as_ldap_mod(&config);

        assert!(mods.contains(&Mod::Replace("sn", hashset!["Smith"])));
        assert!(mods.contains(&Mod::Replace("givenName", hashset!["John"])));
        assert!(mods.contains(&Mod::Replace("mail", hashset!["john.smith@example.com"])));
        assert!(mods.contains(&Mod::Replace("cn", hashset!["testuser"])));
        assert!(mods.contains(&Mod::Replace("sAMAccountName", hashset!["testuser"])));
    }

    #[test]
    fn test_extract_dn_path_various_cases() {
        assert_eq!(
            extract_dn_path("cn=testuser,dc=example,dc=com"),
            Some("dc=example,dc=com".to_string())
        );
        assert_eq!(
            extract_dn_path("uid=abc,ou=users,dc=example,dc=org"),
            Some("ou=users,dc=example,dc=org".to_string())
        );
        assert_eq!(
            extract_dn_path("cn=Test User,dc=example,dc=com"),
            Some("dc=example,dc=com".to_string())
        );
        assert_eq!(
            extract_dn_path("cn=user.name+123,ou=group,dc=example,dc=com"),
            Some("ou=group,dc=example,dc=com".to_string())
        );

        assert_eq!(extract_dn_path("invalid-dn"), None);
        assert_eq!(extract_dn_path("value"), None);

        assert_eq!(extract_dn_path(""), None);

        assert_eq!(extract_dn_path("cn=abc,"), Some("".to_string()));

        assert_eq!(
            extract_dn_path("uid=cde,ou=users,ou=staff,dc=example,dc=org"),
            Some("ou=users,ou=staff,dc=example,dc=org".to_string())
        );

        assert_eq!(extract_dn_path("cn=abc"), None);

        assert_eq!(extract_dn_path("cn=abc"), None);

        assert_eq!(
            extract_dn_path("cn=,dc=example,dc=com"),
            Some("dc=example,dc=com".to_string())
        );

        assert_eq!(
            extract_dn_path("cn=abc=cde,dc=example,dc=com"),
            Some("dc=example,dc=com".to_string())
        );

        assert_eq!(
            extract_dn_path(" cn=abc ,dc=example,dc=com "),
            Some("dc=example,dc=com ".to_string())
        );
    }

    #[sqlx::test]
    async fn test_ldap_sync_allowed_with_empty_sync_groups(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let _ = initialize_current_settings(&pool).await;

        let mut user = make_test_user("testuser");
        user.is_active = true;
        user.password_hash = Some("hash".to_string());
        let user = user.save(&pool).await.unwrap();

        let result = user.ldap_sync_allowed(&pool).await.unwrap();
        assert!(result);
    }

    #[sqlx::test]
    async fn test_ldap_sync_allowed_with_inactive_user(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let _ = initialize_current_settings(&pool).await;

        let mut user = make_test_user("testuser");
        user.is_active = false;
        user.password_hash = Some("hash".to_string());
        let user = user.save(&pool).await.unwrap();

        let result = user.ldap_sync_allowed(&pool).await.unwrap();
        assert!(!result);
    }

    #[sqlx::test]
    async fn test_ldap_sync_allowed_with_unenrolled_user(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let _ = initialize_current_settings(&pool).await;

        let mut user = make_test_user("testuser");
        user.is_active = true;
        user.password_hash = None;
        user.openid_sub = None;
        user.from_ldap = false;
        let user = user.save(&pool).await.unwrap();

        let result = user.ldap_sync_allowed(&pool).await.unwrap();
        assert!(!result);
    }

    #[sqlx::test]
    async fn test_ldap_sync_allowed_with_sync_groups_user_in_group(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let _ = initialize_current_settings(&pool).await;

        let mut user = make_test_user("testuser");
        user.is_active = true;
        user.password_hash = Some("hash".to_string());
        let user = user.save(&pool).await.unwrap();

        let group = Group::new("ldap_sync_group").save(&pool).await.unwrap();
        user.add_to_group(&pool, &group).await.unwrap();

        let mut settings = Settings::get_current_settings();
        settings.ldap_sync_groups = vec!["ldap_sync_group".to_string()];
        update_current_settings(&pool, settings).await.unwrap();

        let result = user.ldap_sync_allowed(&pool).await.unwrap();
        assert!(result);
    }

    #[sqlx::test]
    async fn test_ldap_sync_allowed_with_sync_groups_user_not_in_group(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let _ = initialize_current_settings(&pool).await;

        let mut user = make_test_user("testuser");
        user.is_active = true;
        user.password_hash = Some("hash".to_string());
        let user = user.save(&pool).await.unwrap();

        let _group = Group::new("ldap_sync_group").save(&pool).await.unwrap();
        let other_group = Group::new("other_group").save(&pool).await.unwrap();
        user.add_to_group(&pool, &other_group).await.unwrap();

        let mut settings = Settings::get_current_settings();
        settings.ldap_sync_groups = vec!["ldap_sync_group".to_string()];
        update_current_settings(&pool, settings).await.unwrap();

        let result = user.ldap_sync_allowed(&pool).await.unwrap();
        assert!(!result);
    }

    #[sqlx::test]
    async fn test_ldap_sync_allowed_with_multiple_sync_groups(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let _ = initialize_current_settings(&pool).await;

        let mut user = make_test_user("testuser");
        user.is_active = true;
        user.password_hash = Some("hash".to_string());
        let user = user.save(&pool).await.unwrap();

        let _group1 = Group::new("group1").save(&pool).await.unwrap();
        let group2 = Group::new("group2").save(&pool).await.unwrap();
        let _group3 = Group::new("group3").save(&pool).await.unwrap();

        user.add_to_group(&pool, &group2).await.unwrap();

        let mut settings = Settings::get_current_settings();
        settings.ldap_sync_groups = vec![
            "group1".to_string(),
            "group2".to_string(),
            "group3".to_string(),
        ];
        update_current_settings(&pool, settings).await.unwrap();

        let result = user.ldap_sync_allowed(&pool).await.unwrap();
        assert!(result);
    }

    #[sqlx::test]
    async fn test_ldap_sync_allowed_enrolled_via_openid(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let _ = initialize_current_settings(&pool).await;

        let mut user = make_test_user("testuser");
        user.is_active = true;
        user.password_hash = None;
        user.openid_sub = Some("openid_sub".to_string());
        user.from_ldap = false;
        let user = user.save(&pool).await.unwrap();

        let result = user.ldap_sync_allowed(&pool).await.unwrap();
        assert!(result);
    }

    #[sqlx::test]
    async fn test_ldap_sync_allowed_enrolled_via_ldap(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let _ = initialize_current_settings(&pool).await;

        let mut user = make_test_user("testuser");
        user.is_active = true;
        user.password_hash = None;
        user.openid_sub = None;
        user.from_ldap = true;
        let user = user.save(&pool).await.unwrap();

        let result = user.ldap_sync_allowed(&pool).await.unwrap();
        assert!(result);
    }

    #[sqlx::test]
    async fn test_ldap_sync_allowed_all_conditions_false(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let _ = initialize_current_settings(&pool).await;

        let mut user = make_test_user("testuser");
        user.is_active = false;
        user.password_hash = None;
        user.openid_sub = None;
        user.from_ldap = false;
        let user = user.save(&pool).await.unwrap();

        let _group = Group::new("ldap_sync_group").save(&pool).await.unwrap();

        let mut settings = Settings::get_current_settings();
        settings.ldap_sync_groups = vec!["ldap_sync_group".to_string()];
        update_current_settings(&pool, settings).await.unwrap();

        let result = user.ldap_sync_allowed(&pool).await.unwrap();
        assert!(!result);
    }
}
