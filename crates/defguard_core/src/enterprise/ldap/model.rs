use std::collections::HashSet;

use ldap3::{Mod, SearchEntry};

use super::{LDAPConfig, error::LdapError};
use crate::{db::User, handlers::user::check_username, hashset};

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
                ("uid", hashset![self.username.as_str()]),
            ]);

            if rdn_attr != "cn" {
                attrs.push(("cn", hashset![self.username.as_str()]));
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
        if attrs.iter().all(|(key, _)| *key != username_attr) {
            attrs.push((username_attr, hashset![self.username.as_str()]));
        }

        attrs.push(("objectClass", object_classes));

        debug!("Generated LDAP attributes: {:?}", attrs);

        attrs
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use ldap3::SearchEntry;

    use super::*;

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
    fn test_from_searchentry_success() {
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

    #[test]
    fn test_from_searchentry_without_mobile() {
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

    #[test]
    fn test_from_searchentry_missing_attribute() {
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
}
