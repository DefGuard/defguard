use std::collections::HashSet;

use defguard_common::db::{
    Id,
    models::{Settings, User},
};
use ldap3::{Mod, ResultEntry, SearchEntry};
use sqlx::{PgExecutor, query_as};

use super::{
    LDAPConfig,
    error::{LdapError, sanitize_ldap_string},
};
use crate::{handlers::user::check_username, hashset};

// AD userAccountControl: https://learn.microsoft.com/windows/win32/adschema/a-useraccountcontrol
pub(crate) const UAC_ACCOUNT_DISABLE: u32 = 0x0002;
pub(crate) const UAC_NORMAL_ACCOUNT: u32 = 0x0200;

pub(crate) const LDAP_USER_ACCOUNT_CONTROL_ATTR: &str = "userAccountControl";

#[must_use]
pub(crate) fn uac_is_active(uac: u32) -> bool {
    uac & UAC_ACCOUNT_DISABLE == 0
}

#[must_use]
pub(crate) fn uac_with_active(current: u32, active: bool) -> u32 {
    if active {
        current & !UAC_ACCOUNT_DISABLE
    } else {
        current | UAC_ACCOUNT_DISABLE
    }
}

#[must_use]
pub(crate) fn uac_from_entry(entry: &SearchEntry) -> Option<u32> {
    entry
        .attrs
        .get(LDAP_USER_ACCOUNT_CONTROL_ATTR)
        .and_then(|values| values.first())
        .and_then(|value| value.parse::<u32>().ok())
}

pub(crate) enum UserObjectClass {
    SambaSamAccount,
    InetOrgPerson,
    SimpleSecurityObject,
    User,
}

impl UserObjectClass {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::SambaSamAccount => "sambaSamAccount",
            Self::InetOrgPerson => "inetOrgPerson",
            Self::SimpleSecurityObject => "simpleSecurityObject",
            Self::User => "user",
        }
    }
}

pub(crate) fn user_from_searchentry(
    entry: &SearchEntry,
    username: &str,
    password: Option<&str>,
    config: &LDAPConfig,
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
    // Missing/unparseable userAccountControl falls through with the User::new default (active).
    if config.ldap_uses_ad
        && config.ldap_sync_account_status
        && let Some(uac) = uac_from_entry(entry)
    {
        user.is_active = uac_is_active(uac);
    }
    if let Some(rdn) = extract_rdn_value(&entry.dn) {
        user.ldap_rdn = Some(rdn);
    } else {
        return Err(LdapError::InvalidDN(sanitize_ldap_string(&entry.dn)));
    }
    if let Some(dn_path) = extract_dn_path(&entry.dn) {
        user.ldap_user_path = Some(dn_path);
    } else {
        return Err(LdapError::InvalidDN(sanitize_ldap_string(&entry.dn)));
    }
    // Print the warning only if everything else checks out
    if check_username(username).is_err() {
        warn!(
            "LDAP User \"{username}\" has username that cannot be used in Defguard; change the \
            LDAP username attribute or change the username in LDAP to a valid one"
        );
        return Err(LdapError::InvalidUsername(username.to_owned()));
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

/// Return a vector of LDAP modifications for a given [`User`].
#[must_use]
pub(crate) fn user_as_ldap_mod<I>(user: &User<I>, config: &LDAPConfig) -> Vec<Mod<String>> {
    let obj_classes = config.get_all_user_obj_classes();
    let mut changes = Vec::new();
    if obj_classes
        .iter()
        .any(|e| e == UserObjectClass::InetOrgPerson.name())
        || obj_classes
            .iter()
            .any(|e| e == UserObjectClass::User.name())
    {
        changes.extend_from_slice(&[
            Mod::Replace("sn".to_owned(), hashset![user.last_name.clone()]),
            Mod::Replace("givenName".to_owned(), hashset![user.first_name.clone()]),
            Mod::Replace("mail".to_owned(), hashset![user.email.clone()]),
        ]);

        // Allow renaming the user if the CN is not a part of the RDN
        if !config.get_rdn_attr().eq_ignore_ascii_case("cn") {
            changes.push(Mod::Replace(
                "cn".to_owned(),
                hashset![user.username.clone()],
            ));
        }

        if !config.ldap_username_attr.eq_ignore_ascii_case("uid")
            && !config
                .ldap_user_rdn_attr
                .as_ref()
                .is_some_and(|rdn_attr| rdn_attr.eq_ignore_ascii_case("uid"))
        {
            changes.push(Mod::Replace(
                "uid".to_owned(),
                hashset![user.username.clone()],
            ));
        }

        if let Some(phone) = &user.phone {
            changes.push(Mod::Replace(
                "mobile".to_owned(),
                if phone.is_empty() {
                    HashSet::<String>::new()
                } else {
                    hashset![phone.clone()]
                },
            ));
        }
    } else {
        warn!(
            "No user object class found for user {}, can't generate mods",
            user.username
        );
    }

    if config.ldap_uses_ad && !config.get_rdn_attr().eq_ignore_ascii_case("sAMAccountName") {
        changes.push(Mod::Replace(
            "sAMAccountName".to_owned(),
            hashset![user.username.clone()],
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
            username_attr.to_owned(),
            hashset![user.username.clone()],
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
pub(crate) fn user_as_ldap_attrs<'a, I>(
    user: &'a User<I>,
    ssha_password: &'a str,
    nt_password: &'a str,
    object_classes: HashSet<&'a str>,
    uses_ad: bool,
    username_attr: &'a str,
    rdn_attr: &'a str,
) -> Vec<(&'a str, HashSet<&'a str>)> {
    let mut attrs = Vec::new();
    attrs.push((rdn_attr, hashset![user.ldap_rdn_value()]));
    if object_classes.contains(UserObjectClass::InetOrgPerson.name())
        || object_classes.contains(UserObjectClass::User.name())
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

        if let Some(phone) = &user.phone
            && !phone.is_empty()
        {
            attrs.push(("mobile", hashset![phone.as_str()]));
        }
    }
    if object_classes.contains(UserObjectClass::SimpleSecurityObject.name()) {
        // simpleSecurityObject
        attrs.push(("userPassword", hashset![ssha_password]));
    }
    if object_classes.contains(UserObjectClass::SambaSamAccount.name()) {
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
/// - he is active (not disabled), unless AD account status sync is enabled, in which case
///   disabled users stay in scope so their status can be kept in sync instead of deleting them
/// - he is enrolled, or is an LDAP-origin user whose enrollment is still pending
pub(crate) async fn ldap_sync_allowed_for_user<'e, E>(
    user: &User<Id>,
    executor: E,
) -> sqlx::Result<bool>
where
    E: PgExecutor<'e>,
{
    let settings = Settings::get_current_settings();
    let sync_account_status = settings.ldap_uses_ad && settings.ldap_sync_account_status;
    ldap_sync_allowed_for_user_scoped(
        user,
        executor,
        sync_account_status,
        &settings.ldap_sync_groups,
    )
    .await
}

/// Same as [`ldap_sync_allowed_for_user`] but with the scoping settings passed explicitly.
/// Needed by flows running with settings that differ from the saved ones (LDAP dry run).
pub(crate) async fn ldap_sync_allowed_for_user_scoped<'e, E>(
    user: &User<Id>,
    executor: E,
    sync_account_status: bool,
    sync_groups: &[String],
) -> sqlx::Result<bool>
where
    E: PgExecutor<'e>,
{
    let my_groups = user.member_of(executor).await?;
    Ok(
        (sync_groups.is_empty() || my_groups.iter().any(|g| sync_groups.contains(&g.name)))
            && (user.is_active || sync_account_status)
            && user.is_enrolled_or_ldap_pending(),
    )
}

pub(super) async fn get_users_without_ldap_path<'e, E>(executor: E) -> sqlx::Result<Vec<User<Id>>>
where
    E: PgExecutor<'e>,
{
    query_as!(
        User,
        "SELECT id, username, password_hash, last_name, first_name, email, phone, \
        mfa_enabled, totp_enabled, email_mfa_enabled, totp_secret, email_mfa_secret, \
        mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub, \
        from_ldap, ldap_pass_randomized, ldap_rdn, ldap_user_path, ldap_remote_enrollment_completed, enrollment_pending \
        FROM \"user\" WHERE ldap_user_path IS NULL",
    )
    .fetch_all(executor)
    .await
}

fn get_value_or_error(entry: &SearchEntry, key: &str) -> Result<String, LdapError> {
    match entry.attrs.get(key) {
        Some(values) if !values.is_empty() => Ok(values[0].clone()),
        _ => Err(LdapError::MissingAttribute(key.to_owned())),
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
        dn.get((eq_index + 1)..comma_index).map(str::to_owned)
    } else {
        None
    }
}

/// Returns true only for a SearchResultEntry (LDAP protocol op id 4).
///
/// Referrals (id 19), intermediate responses (id 25), and any other result type
/// are rejected. This mirrors the id that `SearchEntry::try_construct` requires, so a
/// `true` result guarantees the entry will decode.
#[must_use]
pub(super) fn is_search_entry(entry: &ResultEntry) -> bool {
    entry.0.id == 4
}

/// Extract the remaining part of the distinguished name after the first comma, for example:
/// `cn=user,dc=example,dc=com` should return `dc=example,dc=com`.
#[must_use]
pub(crate) fn extract_dn_path(dn: &str) -> Option<String> {
    if let Some(parts) = dn.split_once(',') {
        let path = parts.1.to_owned();
        debug!("Extracted DN path '{path}' from DN '{dn}'");
        Some(path)
    } else {
        warn!("Failed to extract DN path from '{dn}': no comma found");
        None
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use ldap3::{
        ResultEntry, SearchEntry,
        asn1::{PL, StructureTag, TagClass},
    };

    use super::*;

    const UAC_DONT_EXPIRE_PASSWORD: u32 = 0x10000;

    fn result_entry(id: u64) -> ResultEntry {
        ResultEntry::new(StructureTag {
            class: TagClass::Application,
            id,
            payload: PL::C(Vec::new()),
        })
    }

    #[test]
    fn is_search_entry_accepts_only_real_entries() {
        // id 4 is a SearchResultEntry, the only type SearchEntry::try_construct accepts.
        assert!(is_search_entry(&result_entry(4)));
        // id 19 is a referral, id 25 an intermediate response.
        assert!(!is_search_entry(&result_entry(19)));
        assert!(!is_search_entry(&result_entry(25)));
        // Any other response
        assert!(!is_search_entry(&result_entry(7)));
        assert!(!is_search_entry(&result_entry(12)));
        assert!(!is_search_entry(&result_entry(45)));
    }

    #[test]
    fn test_uac_is_active() {
        // Regular enabled account.
        assert!(uac_is_active(UAC_NORMAL_ACCOUNT));
        // Enabled account with extra flags set.
        assert!(uac_is_active(UAC_NORMAL_ACCOUNT | UAC_DONT_EXPIRE_PASSWORD));
        // Disabled account.
        assert!(!uac_is_active(UAC_NORMAL_ACCOUNT | UAC_ACCOUNT_DISABLE));
        assert!(!uac_is_active(
            UAC_NORMAL_ACCOUNT | UAC_DONT_EXPIRE_PASSWORD | UAC_ACCOUNT_DISABLE
        ));
    }

    #[test]
    fn test_uac_with_active_preserves_other_flags() {
        let enabled = UAC_NORMAL_ACCOUNT | UAC_DONT_EXPIRE_PASSWORD;
        // Disabling only sets the ACCOUNTDISABLE bit, other flags remain.
        let disabled = uac_with_active(enabled, false);
        assert_eq!(disabled, enabled | UAC_ACCOUNT_DISABLE);
        assert!(disabled & UAC_DONT_EXPIRE_PASSWORD != 0);
        // Re-enabling clears only the ACCOUNTDISABLE bit, other flags remain.
        let reenabled = uac_with_active(disabled, true);
        assert_eq!(reenabled, enabled);
        assert!(reenabled & UAC_DONT_EXPIRE_PASSWORD != 0);
        // Idempotent when already in the desired state.
        assert_eq!(uac_with_active(enabled, true), enabled);
        assert_eq!(uac_with_active(disabled, false), disabled);
    }

    fn ad_entry_with_uac(uac: Option<&str>) -> SearchEntry {
        let mut attrs = HashMap::new();
        attrs.insert("sn".to_owned(), vec!["lastname".to_owned()]);
        attrs.insert("givenName".to_owned(), vec!["firstname".to_owned()]);
        attrs.insert("mail".to_owned(), vec!["user@example.com".to_owned()]);
        if let Some(uac) = uac {
            attrs.insert(
                LDAP_USER_ACCOUNT_CONTROL_ATTR.to_owned(),
                vec![uac.to_owned()],
            );
        }
        SearchEntry {
            dn: "cn=user,dc=example,dc=com".to_owned(),
            attrs,
            bin_attrs: HashMap::new(),
        }
    }

    #[test]
    fn test_user_from_searchentry_reads_ad_disabled_status() {
        let ad_config = LDAPConfig {
            ldap_uses_ad: true,
            ldap_sync_account_status: true,
            ..LDAPConfig::default()
        };

        // Disabled in AD -> inactive in Defguard.
        let entry = ad_entry_with_uac(Some("514")); // 512 | ACCOUNTDISABLE
        let user = user_from_searchentry(&entry, "user", None, &ad_config).unwrap();
        assert!(!user.is_active);

        // Enabled in AD -> active in Defguard.
        let entry = ad_entry_with_uac(Some("512"));
        let user = user_from_searchentry(&entry, "user", None, &ad_config).unwrap();
        assert!(user.is_active);

        // Missing userAccountControl -> defaults to active.
        let entry = ad_entry_with_uac(None);
        let user = user_from_searchentry(&entry, "user", None, &ad_config).unwrap();
        assert!(user.is_active);
    }

    #[test]
    fn test_user_from_searchentry_ignores_uac_when_disabled() {
        // Account status sync off: userAccountControl is ignored, user stays active.
        let entry = ad_entry_with_uac(Some("514"));
        let ad_no_status = LDAPConfig {
            ldap_uses_ad: true,
            ldap_sync_account_status: false,
            ..LDAPConfig::default()
        };
        let user = user_from_searchentry(&entry, "user", None, &ad_no_status).unwrap();
        assert!(user.is_active);

        // Non-AD LDAP with the flag on: still ignored (AD only).
        let non_ad = LDAPConfig {
            ldap_uses_ad: false,
            ldap_sync_account_status: true,
            ..LDAPConfig::default()
        };
        let user = user_from_searchentry(&entry, "user", None, &non_ad).unwrap();
        assert!(user.is_active);
    }

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
        let empty_attrs = Vec::new();
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
