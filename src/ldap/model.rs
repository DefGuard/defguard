use std::collections::HashSet;

use ldap3::{Mod, SearchEntry};

use super::{error::LdapError, LDAPConfig};
use crate::{
    db::{Settings, User},
    hashset,
};

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
        user.ldap_linked = true;
        Ok(user)
    }
}

impl<I> User<I> {
    #[must_use]
    pub fn as_ldap_mod(&self, config: &LDAPConfig) -> Vec<Mod<&str>> {
        let mut changes = vec![
            Mod::Replace("sn", hashset![self.last_name.as_str()]),
            Mod::Replace("givenName", hashset![self.first_name.as_str()]),
            Mod::Replace("mail", hashset![self.email.as_str()]),
        ];
        if let Some(phone) = &self.phone {
            if phone.is_empty() {
                changes.push(Mod::Replace("mobile", HashSet::new()));
            } else {
                changes.push(Mod::Replace("mobile", hashset![phone.as_str()]));
            }
        }
        // Be careful when changing naming attribute (the one in distingushed name)
        if config.ldap_username_attr != "cn" {
            changes.push(Mod::Replace("cn", hashset![self.username.as_str()]));
        }
        if config.ldap_username_attr != "uid" {
            changes.push(Mod::Replace("uid", hashset![self.username.as_str()]));
        }
        changes
    }

    #[must_use]
    pub fn as_ldap_attrs<'a>(
        &'a self,
        ssha_password: &'a str,
        nt_password: &'a str,
    ) -> Vec<(&'a str, HashSet<&'a str>)> {
        let settings = Settings::get_current_settings();
        let mut attrs = vec![
            (
                "objectClass",
                hashset!["inetOrgPerson", "simpleSecurityObject", "sambaSamAccount"],
            ),
            // inetOrgPerson
            ("cn", hashset![self.username.as_str()]),
            ("sn", hashset![self.last_name.as_str()]),
            ("givenName", hashset![self.first_name.as_str()]),
            ("mail", hashset![self.email.as_str()]),
            ("uid", hashset![self.username.as_str()]),
            // simpleSecurityObject
            ("userPassword", hashset![ssha_password]),
        ];
        if settings.ldap_samba_enabled {
            // sambaSamAccount
            attrs.push(("sambaSID", hashset!["0"]));
            attrs.push(("sambaNTPassword", hashset![nt_password]));
        }
        if let Some(phone) = &self.phone {
            if !phone.is_empty() {
                attrs.push(("mobile", hashset![phone.as_str()]));
            }
        }
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
pub(crate) fn extract_dn_value(dn: &str) -> Option<String> {
    if let (Some(eq_index), Some(comma_index)) = (dn.find('='), dn.find(',')) {
        dn.get((eq_index + 1)..comma_index).map(ToString::to_string)
    } else {
        None
    }
}

impl<'a> From<&'a User> for Vec<(&'a str, HashSet<&'a str>)> {
    fn from(user: &'a User) -> Self {
        let mut attrs = vec![
            (
                "objectClass",
                hashset!["inetOrgPerson", "simpleSecurityObject"],
            ),
            ("cn", hashset![user.username.as_str()]),
            ("sn", hashset![user.last_name.as_str()]),
            ("givenName", hashset![user.first_name.as_str()]),
            ("mail", hashset![user.email.as_str()]),
            ("uid", hashset![user.username.as_str()]),
        ];
        if let Some(ref phone) = user.phone {
            attrs.push(("mobile", hashset![phone.as_str()]));
        }
        attrs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ldap3::SearchEntry;
    use std::collections::HashMap;

    #[test]
    fn test_extract_dn_value() {
        assert_eq!(
            extract_dn_value("cn=testuser,dc=example,dc=com"),
            Some("testuser".to_string())
        );
        assert_eq!(
            extract_dn_value("cn=Test User,dc=example,dc=com"),
            Some("Test User".to_string())
        );
        assert_eq!(
            extract_dn_value("cn=user.name+123,dc=example,dc=com"),
            Some("user.name+123".to_string())
        );
        assert_eq!(extract_dn_value("invalid-dn"), None);
        assert_eq!(extract_dn_value("cn=onlyvalue"), None);
        assert_eq!(
            extract_dn_value("cn=,dc=example,dc=com"),
            Some("".to_string())
        );
        assert_eq!(extract_dn_value(""), None);
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
        assert!(user.ldap_linked);
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
        assert!(user.ldap_linked);
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
}
