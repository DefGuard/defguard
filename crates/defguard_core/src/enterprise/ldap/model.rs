use std::collections::HashSet;

use defguard_common::db::{
    Id,
    models::{Settings, User},
};
use ldap3::{Mod, SearchEntry};
use sqlx::{Error as SqlxError, PgExecutor};

use super::{LDAPConfig, error::LdapError};
use crate::{handlers::user::check_username, hashset};

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

pub(crate) fn user_from_searchentry(
    entry: &SearchEntry,
    username: &str,
    password: Option<&str>,
) -> Result<User, LdapError> {
    let mut user = User::new(
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
            "LDAP User \"{username}\" has username that cannot be used in Defguard, \
                change the LDAP username attribute or change the username in LDAP to a valid one",
        );
        return Err(LdapError::InvalidUsername(username.to_string()));
    }
    Ok(user)
}

pub(crate) fn update_from_ldap_user<I>(user: &mut User<I>, ldap_user: &User, config: &LDAPConfig) {
    user.last_name.clone_from(&ldap_user.last_name);
    user.first_name.clone_from(&ldap_user.first_name);
    user.email.clone_from(&ldap_user.email);
    user.phone.clone_from(&ldap_user.phone);
    // It should be ok to update the username if we are not using it in the DN (not as RDN)
    if config.using_username_as_rdn() {
        debug!(
            "Not updating username {} from LDAP because it is used as RDN",
            user.username
        );
    } else {
        user.username.clone_from(&ldap_user.username);
    }
}

#[must_use]
pub fn user_as_ldap_mod<'a, I>(user: &'a User<I>, config: &'a LDAPConfig) -> Vec<Mod<&'a str>> {
    let obj_classes = config.get_all_user_obj_classes();
    let mut changes = vec![];
    if obj_classes.contains(&UserObjectClass::InetOrgPerson.into())
        || obj_classes.contains(&UserObjectClass::User.into())
    {
        changes.extend_from_slice(&[
            Mod::Replace("sn", hashset![user.last_name.as_str()]),
            Mod::Replace("givenName", hashset![user.first_name.as_str()]),
            Mod::Replace("mail", hashset![user.email.as_str()]),
        ]);

        // Allow renaming the user if the CN is not a part of the RDN
        if !config.get_rdn_attr().eq_ignore_ascii_case("cn") {
            changes.push(Mod::Replace("cn", hashset![user.username.as_str()]));
        }

        if !config.ldap_username_attr.eq_ignore_ascii_case("uid")
            && !config
                .ldap_user_rdn_attr
                .as_ref()
                .is_some_and(|rdn_attr| rdn_attr.eq_ignore_ascii_case("uid"))
        {
            changes.push(Mod::Replace("uid", hashset![user.username.as_str()]));
        }

        if let Some(phone) = &user.phone {
            if phone.is_empty() {
                changes.push(Mod::Replace("mobile", HashSet::new()));
            } else {
                changes.push(Mod::Replace("mobile", hashset![phone.as_str()]));
            }
        }
    } else {
        warn!(
            "No user object class found for user {}, can't generate mods",
            user.username
        );
    }

    if config.ldap_uses_ad && !config.get_rdn_attr().eq_ignore_ascii_case("sAMAccountName") {
        changes.push(Mod::Replace(
            "sAMAccountName",
            hashset![user.username.as_str()],
        ));
    }

    let username_attr = config.ldap_username_attr.as_str();
    // Add anything the user provided, if we haven't already added it AND it's not the same as
    // the RDN.
    if !username_attr.eq_ignore_ascii_case("sAMAccountName")
        && !username_attr.eq_ignore_ascii_case("cn")
        && !config
            .ldap_user_rdn_attr
            .as_ref()
            .is_some_and(|rdn_attr| rdn_attr.eq_ignore_ascii_case(username_attr))
    {
        changes.push(Mod::Replace(
            username_attr,
            hashset![user.username.as_str()],
        ));
    }

    changes
}

// check if key is already in attrs, if not return false
#[cfg(test)]
pub(crate) fn in_attrs<'a>(attrs: &'a Vec<(&'a str, HashSet<&'a str>)>, key: &str) -> bool {
    attrs.iter().any(|(k, _)| k.eq_ignore_ascii_case(key))
}

#[cfg(not(test))]
fn in_attrs<'a>(attrs: &'a Vec<(&'a str, HashSet<&'a str>)>, key: &str) -> bool {
    attrs.iter().any(|(k, _)| k.eq_ignore_ascii_case(key))
}

#[must_use]
pub fn user_as_ldap_attrs<'a, I>(
    user: &'a User<I>,
    ssha_password: &'a str,
    nt_password: &'a str,
    object_classes: HashSet<&'a str>,
    uses_ad: bool,
    username_attr: &'a str,
    rdn_attr: &'a str,
) -> Vec<(&'a str, HashSet<&'a str>)> {
    let mut attrs = vec![];
    attrs.push((rdn_attr, hashset![user.ldap_rdn_value()]));
    if object_classes.contains(UserObjectClass::InetOrgPerson.into())
        || object_classes.contains(UserObjectClass::User.into())
    {
        attrs.extend_from_slice(&[
            ("sn", hashset![user.last_name.as_str()]),
            ("givenName", hashset![user.first_name.as_str()]),
            ("mail", hashset![user.email.as_str()]),
        ]);

        if !in_attrs(&attrs, "cn") {
            attrs.push(("cn", hashset![user.username.as_str()]));
        }

        if !in_attrs(&attrs, "uid") {
            attrs.push(("uid", hashset![user.username.as_str()]));
        }

        if let Some(phone) = &user.phone {
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
        attrs.push(("sAMAccountName", hashset![user.username.as_str()]));
    }

    // Add the username attr and RDN if we haven't already added it
    if !in_attrs(&attrs, username_attr) {
        attrs.push((username_attr, hashset![user.username.as_str()]));
    }

    attrs.push(("objectClass", object_classes));

    debug!("Generated LDAP attributes: {attrs:?}");

    attrs
}

/// Updates the LDAP RDN value of the user in Defguard, if Defguard uses the usernames as RDN.
pub(crate) fn maybe_update_rdn<I>(user: &mut User<I>) {
    debug!("Updating RDN for user {} in Defguard", user.username);
    let settings = Settings::get_current_settings();
    if settings.ldap_using_username_as_rdn() {
        debug!("The user's username is being used as the RDN, setting it to username");
        user.ldap_rdn = Some(user.username.clone());
    } else {
        debug!("The user's username is NOT being used as the RDN, skipping update");
    }
}

/// User is syncable with LDAP if:
/// - he is in a group that is allowed to be synced or no such groups are configured
/// - he is active (not disabled)
/// - he is enrolled
pub(crate) async fn ldap_sync_allowed_for_user<'e, E>(
    user: &User<Id>,
    executor: E,
) -> Result<bool, SqlxError>
where
    E: PgExecutor<'e>,
{
    let sync_groups = Settings::get_current_settings().ldap_sync_groups;
    let my_groups = user.member_of(executor).await?;
    Ok(
        (sync_groups.is_empty() || my_groups.iter().any(|g| sync_groups.contains(&g.name)))
            && user.is_active
            && user.is_enrolled(),
    )
}

pub(super) async fn get_users_without_ldap_path<'e, E>(
    executor: E,
) -> Result<Vec<User<Id>>, SqlxError>
where
    E: PgExecutor<'e>,
{
    let users = sqlx::query_as!(
        User,
        "
            SELECT id, username, password_hash, last_name, first_name, email, phone, \
            mfa_enabled, totp_enabled, email_mfa_enabled, totp_secret, email_mfa_secret, \
            mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub, \
            from_ldap, ldap_pass_randomized, ldap_rdn, ldap_user_path, enrollment_pending \
            FROM \"user\" WHERE ldap_user_path IS NULL
            ",
    )
    .fetch_all(executor)
    .await?;

    Ok(users.into_iter().map(Into::into).collect())
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
        debug!("Extracted DN path '{path}' from DN '{dn}'");
        Some(path)
    } else {
        warn!("Failed to extract DN path from '{dn}': no comma found");
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_attrs() {
        // Create test attributes with mixed case keys
        let attrs = vec![
            ("cn", hashset!["user1"]),
            ("Mail", hashset!["user@example.com"]),
            ("PHONE", hashset!["123456789"]),
            ("givenName", hashset!["John"]),
        ];

        // Test exact case match
        assert!(in_attrs(&attrs, "cn"));
        assert!(in_attrs(&attrs, "Mail"));
        assert!(in_attrs(&attrs, "PHONE"));
        assert!(in_attrs(&attrs, "givenName"));

        // Test case-insensitive matching
        assert!(in_attrs(&attrs, "CN"));
        assert!(in_attrs(&attrs, "cn"));
        assert!(in_attrs(&attrs, "mail"));
        assert!(in_attrs(&attrs, "MAIL"));
        assert!(in_attrs(&attrs, "phone"));
        assert!(in_attrs(&attrs, "Phone"));
        assert!(in_attrs(&attrs, "GIVENNAME"));
        assert!(in_attrs(&attrs, "givenname"));

        // Test non-existent attributes
        assert!(!in_attrs(&attrs, "nonexistent"));
        assert!(!in_attrs(&attrs, "sn"));
        assert!(!in_attrs(&attrs, "uid"));

        // Test empty attributes vector
        let empty_attrs = vec![];
        assert!(!in_attrs(&empty_attrs, "cn"));
        assert!(!in_attrs(&empty_attrs, "any"));

        // Test with empty string key
        assert!(!in_attrs(&attrs, ""));

        // Test with attributes that have empty values (should still match on key)
        let attrs_with_empty_values = vec![
            ("cn", HashSet::new()),
            ("mail", hashset!["test@example.com"]),
        ];
        assert!(in_attrs(&attrs_with_empty_values, "cn"));
        assert!(in_attrs(&attrs_with_empty_values, "CN"));
        assert!(in_attrs(&attrs_with_empty_values, "mail"));
        assert!(!in_attrs(&attrs_with_empty_values, "phone"));
    }
}
