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

// TODO: This struct is similar to `GroupInfo`, so maybe use one?
// pub(crate) struct Group {
//     pub name: String,
//     pub members: Vec<String>,
// }

// impl Group {
//     #[must_use]
//     pub(crate) fn from_searchentry(entry: &SearchEntry, config: &LDAPConfig) -> Self {
//         Self {
//             name: get_value_or_default(entry, &config.ldap_groupname_attr),
//             members: match entry.attrs.get(&config.ldap_group_member_attr) {
//                 Some(members) => members
//                     .iter()
//                     .filter_map(|member| extract_dn_value(member))
//                     .collect(),
//                 None => Vec::new(),
//             },
//         }
//     }
// }

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
pub fn extract_dn_value(dn: &str) -> Option<String> {
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
