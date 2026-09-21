use std::{
    collections::{HashMap, HashSet},
    fmt,
    hash::{Hash, Hasher},
    ops::Deref,
};

use defguard_common::db::{
    Id,
    models::{Settings, User},
};
use ldap3::{Mod, ResultEntry};
use sqlx::{PgExecutor, query_as};

use super::{
    LDAPConfig,
    dn::{find_unescaped_separator, unescape_value},
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
pub(crate) fn uac_from_entry(entry: &LdapEntry) -> Option<u32> {
    entry
        .first(LDAP_USER_ACCOUNT_CONTROL_ATTR)
        .and_then(|value| value.parse::<u32>().ok())
}

/// A distinguished name that compares and hashes ignoring case.
#[derive(Clone, Debug, Eq)]
pub(crate) struct Dn(String);

impl PartialEq for Dn {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }
}

impl Hash for Dn {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_lowercase().hash(state);
    }
}

impl Deref for Dn {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Dn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for Dn {
    fn from(dn: String) -> Self {
        Self(dn)
    }
}

impl From<&str> for Dn {
    fn from(dn: &str) -> Self {
        Self(dn.to_owned())
    }
}

/// An LDAP entry whose attribute reads ignore case per RFC 4512 section 2.5.
#[derive(Debug)]
pub(crate) struct LdapEntry {
    pub(crate) dn: Dn,
    attrs: HashMap<String, Vec<String>>,
}

impl LdapEntry {
    pub(crate) fn new(dn: Dn, attrs: impl IntoIterator<Item = (String, Vec<String>)>) -> Self {
        let mut folded: HashMap<String, Vec<String>> = HashMap::new();
        for (name, values) in attrs {
            folded
                .entry(name.to_lowercase())
                .or_default()
                .extend(values);
        }
        Self { dn, attrs: folded }
    }

    #[must_use]
    pub(crate) fn values(&self, attr: &str) -> Option<&[String]> {
        self.attrs.get(&attr.to_lowercase()).map(Vec::as_slice)
    }

    #[must_use]
    pub(crate) fn first(&self, attr: &str) -> Option<&str> {
        self.values(attr)
            .and_then(<[String]>::first)
            .map(String::as_str)
    }
}

/// Matches object class names case-insensitively per RFC 4512.
#[must_use]
pub(super) fn has_obj_class(classes: &[&str], name: &str) -> bool {
    classes.iter().any(|c| c.eq_ignore_ascii_case(name))
}

/// Matches any of the given object classes case-insensitively per RFC 4512.
#[must_use]
pub(super) fn has_any_obj_class(classes: &[&str], names: &[UserObjectClass]) -> bool {
    classes
        .iter()
        .any(|c| names.iter().any(|n| c.eq_ignore_ascii_case(n.name())))
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
    entry: &LdapEntry,
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
        entry.first("mobile").map(str::to_owned),
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
    let obj_class_names: Vec<&str> = obj_classes.iter().map(String::as_str).collect();
    let mut changes = Vec::new();
    if has_any_obj_class(
        &obj_class_names,
        &[UserObjectClass::InetOrgPerson, UserObjectClass::User],
    ) {
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

pub(crate) fn in_attrs<'a>(attrs: &'a Vec<(&'a str, HashSet<&'a str>)>, key: &str) -> bool {
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
    let obj_class_names: Vec<&str> = object_classes.iter().copied().collect();
    if has_any_obj_class(
        &obj_class_names,
        &[UserObjectClass::InetOrgPerson, UserObjectClass::User],
    ) {
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
    if has_obj_class(
        &obj_class_names,
        UserObjectClass::SimpleSecurityObject.name(),
    ) {
        // simpleSecurityObject
        attrs.push(("userPassword", hashset![ssha_password]));
    }
    if has_obj_class(&obj_class_names, UserObjectClass::SambaSamAccount.name()) {
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
    Ok((sync_groups.is_empty()
        || my_groups
            .iter()
            .any(|g| group_in_list(sync_groups, &g.name)))
        && (user.is_active || sync_account_status)
        && user.is_enrolled_or_ldap_pending())
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

fn get_value_or_error(entry: &LdapEntry, key: &str) -> Result<String, LdapError> {
    entry
        .first(key)
        .map(str::to_owned)
        .ok_or_else(|| LdapError::MissingAttribute(key.to_owned()))
}

#[must_use]
pub(super) fn group_in_list(groups: &[String], name: &str) -> bool {
    groups
        .iter()
        .any(|g| g.to_lowercase() == name.to_lowercase())
}

/// Rewrites a DN the server sent, for comparison against `LDAPConfig::user_dn`.
///
/// A server may escape the same name as either `\,` or `\2c`, so a `member` value and the DN
/// Defguard rebuilds from a user's stored RDN and path can differ byte for byte. Splitting the name
/// and putting it back the same way lands both on one string. What matters is that the spelling is
/// the same on both sides, not that it is the correct one.
#[must_use]
pub(crate) fn dn_match_key(dn: &str, config: &LDAPConfig) -> Dn {
    match (extract_rdn_value(dn), extract_dn_path(dn)) {
        (Some(rdn), Some(path)) => config.dn_from_parts(&rdn, &path),
        _ => dn.into(),
    }
}

/// Returns the unescaped value of the first component, so `cn=Doe\, John,ou=x` gives `Doe, John`.
#[must_use]
pub(crate) fn extract_rdn_value(dn: &str) -> Option<String> {
    let eq_index = find_unescaped_separator(dn, b'=')?;
    let comma_index = find_unescaped_separator(dn, b',')?;
    if eq_index >= comma_index {
        return None;
    }

    dn.get((eq_index + 1)..comma_index).and_then(unescape_value)
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

/// Returns the part after the first unescaped comma, so `cn=user,dc=example` gives `dc=example`.
#[must_use]
pub(crate) fn extract_dn_path(dn: &str) -> Option<String> {
    let Some(comma_index) = find_unescaped_separator(dn, b',') else {
        warn!("Failed to extract DN path from '{dn}': no comma found");
        return None;
    };

    let path = dn[(comma_index + 1)..].to_owned();
    debug!("Extracted DN path '{path}' from DN '{dn}'");
    Some(path)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use ldap3::{
        ResultEntry,
        asn1::{PL, StructureTag, TagClass},
    };

    use super::*;

    const UAC_DONT_EXPIRE_PASSWORD: u32 = 0x10000;

    fn result_entry(id: u64) -> ResultEntry {
        ResultEntry::new(StructureTag {
            class: TagClass::Application,
            id,
            payload: PL::C(vec![]),
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
    fn test_group_in_list_folds_non_ascii() {
        // Latin letters with diacritics fold like the server's caseIgnoreMatch, which
        // the previous ASCII-only comparison did not.
        assert!(group_in_list(&["Örgü".to_owned()], "örgü"));
        assert!(group_in_list(&["ŁÓDŹ".to_owned()], "łódź"));
        // Distinct letters must not fold together.
        assert!(!group_in_list(&["Örgü".to_owned()], "Orgu"));
    }

    #[test]
    fn test_dn_eq_and_hash_ignore_case() {
        let server_dn = Dn::from("CN=testuser,OU=Users,DC=example,DC=com");
        let built_dn = Dn::from("cn=testuser,ou=users,dc=example,dc=com");
        assert_eq!(server_dn, built_dn);
        assert!(HashSet::from([server_dn.clone()]).contains(&built_dn));
        assert!(
            HashSet::from([Dn::from("cn=Ünal,dc=example,dc=com")])
                .contains(&Dn::from("cn=ünal,dc=example,dc=com"))
        );
        assert_eq!(&*server_dn, "CN=testuser,OU=Users,DC=example,DC=com");
        assert_ne!(
            Dn::from("cn=testuser,dc=example,dc=com"),
            Dn::from("cn=testuser2,dc=example,dc=com")
        );
    }

    #[test]
    fn test_ldap_entry_merges_duplicate_attribute_spellings() {
        let entry = LdapEntry::new(
            "cn=testuser,dc=example,dc=com".into(),
            [
                ("cn".to_owned(), vec!["testuser".to_owned()]),
                ("CN".to_owned(), vec!["TestUser".to_owned()]),
            ],
        );
        assert_eq!(entry.values("Cn").unwrap(), ["testuser", "TestUser"]);
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

    fn ad_entry_with_uac(uac: Option<&str>) -> LdapEntry {
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
        LdapEntry::new("cn=user,dc=example,dc=com".into(), attrs)
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
