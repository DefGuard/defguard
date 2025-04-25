#[cfg(test)]
use std::collections::HashMap;
use std::{collections::HashSet, future::Future};

#[cfg(not(test))]
use ldap3::Ldap;
use ldap3::{ldap_escape, Mod, SearchEntry};
use model::UserObjectClass;
use rand::Rng;
use sqlx::PgPool;
use sync::{get_ldap_sync_status, is_ldap_desynced, set_ldap_sync_status, SyncStatus};

use self::error::LdapError;
use crate::{
    db::{self, models::settings::update_current_settings, Id, Settings, User},
    enterprise::{is_enterprise_enabled, limits::update_counts},
};

#[cfg(not(test))]
pub mod client;
pub mod error;
pub mod hash;
pub mod model;
pub mod sync;
#[cfg(test)]
pub mod test_client;
pub mod utils;

pub(crate) async fn do_ldap_sync(pool: &PgPool) -> Result<(), LdapError> {
    debug!("Starting LDAP sync, if enabled");
    let mut settings = Settings::get_current_settings();
    if !is_enterprise_enabled() {
        info!("Enterprise features are disabled, not performing LDAP sync and automatically disabling it");
        settings.ldap_sync_enabled = false;
        update_current_settings(pool, settings).await?;
        return Err(LdapError::EnterpriseDisabled("LDAP sync".to_string()));
    }

    // Mark as out of sync only if we can't propagate changes to LDAP, as it
    // doesn't matter for the sync status if we can't pull changes.
    if !settings.ldap_enabled {
        debug!("LDAP is disabled, not performing LDAP sync");
        if get_ldap_sync_status() == SyncStatus::InSync {
            set_ldap_sync_status(SyncStatus::OutOfSync, pool).await?;
        }
        return Ok(());
    }

    if !settings.ldap_sync_enabled {
        debug!("LDAP sync is disabled, not performing LDAP sync");
        return Ok(());
    }

    if is_ldap_desynced() {
        info!("LDAP is considered to be desynced, doing a full sync");
    } else {
        info!("Ldap is not considered to be desynced, doing an incremental sync");
        debug!("Because of incremental sync, LDAP authority will be selected to pull changes from LDAP");
    }

    let mut ldap_connection = match LDAPConnection::create().await {
        Ok(connection) => connection,
        Err(err) => {
            set_ldap_sync_status(SyncStatus::OutOfSync, pool).await?;
            return Err(err);
        }
    };

    if let Err(err) = ldap_connection.sync(pool, is_ldap_desynced()).await {
        set_ldap_sync_status(SyncStatus::OutOfSync, pool).await?;
        return Err(err);
    } else {
        set_ldap_sync_status(SyncStatus::InSync, pool).await?;
    };

    let _ = update_counts(pool).await;

    info!("LDAP sync completed");

    Ok(())
}

/// Convenience function to run a function that performs an LDAP operation and handle the result
/// appropriately, setting the LDAP sync status to Desynced if an error is encountered.
pub(crate) async fn with_ldap_status<T, F>(pool: &PgPool, f: F) -> Result<T, LdapError>
where
    F: Future<Output = Result<T, LdapError>>,
{
    let settings = Settings::get_current_settings();
    if !is_enterprise_enabled() {
        info!("Enterprise features are disabled, not performing LDAP operation");
        set_ldap_sync_status(SyncStatus::OutOfSync, pool).await?;
        return Err(LdapError::EnterpriseDisabled("LDAP".to_string()));
    }

    if !settings.ldap_enabled {
        debug!("LDAP is disabled, not performing LDAP operation");
        set_ldap_sync_status(SyncStatus::OutOfSync, pool).await?;
        return Err(LdapError::MissingSettings("LDAP is disabled".into()));
    }

    if settings.ldap_sync_enabled && get_ldap_sync_status() == SyncStatus::OutOfSync {
        warn!("LDAP is considered to be desynced, not performing LDAP operation");
        return Err(LdapError::Desynced);
    }

    match f.await {
        Ok(result) => Ok(result),
        Err(e) => {
            warn!("Encountered an error while performing LDAP operation: {e:?}");
            if let Err(status_err) = set_ldap_sync_status(SyncStatus::OutOfSync, pool).await {
                warn!("Failed to update LDAP sync status: {:?}", status_err);
            }

            Err(e)
        }
    }
}

#[macro_export]
macro_rules! hashset {
    ( $( $element:expr ),* ) => {
        {
            let mut map = HashSet::new();
            $(
                map.insert($element);
            )*
            map
        }
    };
}

#[derive(Clone)]
pub struct LDAPConfig {
    pub ldap_bind_username: String,
    pub ldap_group_search_base: String,
    pub ldap_user_search_base: String,
    pub ldap_user_obj_class: String,
    pub ldap_group_obj_class: String,
    pub ldap_username_attr: String,
    pub ldap_groupname_attr: String,
    pub ldap_group_member_attr: String,
    pub ldap_member_attr: String,
    pub ldap_user_auxiliary_obj_classes: Vec<String>,
    pub ldap_uses_ad: bool,
    pub ldap_user_rdn_attr: Option<String>,
    pub ldap_sync_groups: Vec<String>,
}

#[cfg(test)]
impl Default for LDAPConfig {
    fn default() -> Self {
        Self {
            ldap_bind_username: "admin".to_string(),
            ldap_group_search_base: "ou=groups,dc=example,dc=com".to_string(),
            ldap_user_search_base: "ou=users,dc=example,dc=com".to_string(),
            ldap_user_obj_class: "inetOrgPerson".to_string(),
            ldap_group_obj_class: "groupOfUniqueNames".to_string(),
            ldap_username_attr: "cn".to_string(),
            ldap_groupname_attr: "cn".to_string(),
            ldap_group_member_attr: "uniqueMember".to_string(),
            ldap_member_attr: "memberOf".to_string(),
            ldap_user_auxiliary_obj_classes: vec!["simpleSecurityObject".to_string()],
            ldap_uses_ad: false,
            ldap_user_rdn_attr: None,
            ldap_sync_groups: Vec::new(),
        }
    }
}

impl LDAPConfig {
    #[must_use]
    pub(crate) fn get_rdn_attr(&self) -> &str {
        let attr = self
            .ldap_user_rdn_attr
            .as_deref()
            .unwrap_or(&self.ldap_username_attr)
            .trim();

        if attr.is_empty() {
            &self.ldap_username_attr
        } else {
            attr
        }
    }

    /// Constructs user distinguished name.
    #[must_use]
    pub(crate) fn user_dn(&self, user_rdn_value: &str) -> String {
        format!(
            "{}={user_rdn_value},{}",
            self.get_rdn_attr(),
            self.ldap_user_search_base,
        )
    }

    /// Constructs group distinguished name.
    #[must_use]
    pub(crate) fn group_dn(&self, groupname: &str) -> String {
        format!(
            "{}={groupname},{}",
            self.ldap_groupname_attr, self.ldap_group_search_base,
        )
    }

    #[must_use]
    pub(crate) fn get_all_user_obj_classes(&self) -> Vec<String> {
        let mut obj_classes = vec![self.ldap_user_obj_class.clone()];
        obj_classes.extend(self.ldap_user_auxiliary_obj_classes.to_vec());
        obj_classes
    }

    pub(crate) fn using_username_as_rdn(&self) -> bool {
        // RDN not set = username is used as RDN
        // RDN set = username is used as RDN if they are the same
        self.ldap_user_rdn_attr
            .as_deref()
            .is_none_or(|rdn| rdn == self.ldap_username_attr)
    }
}

impl TryFrom<Settings> for LDAPConfig {
    type Error = LdapError;

    fn try_from(settings: Settings) -> Result<LDAPConfig, LdapError> {
        // Helper function to validate non-empty string settings
        fn validate_string_setting(
            value: Option<String>,
            setting_name: &str,
        ) -> Result<String, LdapError> {
            match value {
                Some(s) if !s.is_empty() => Ok(s),
                Some(_) => Err(LdapError::MissingSettings(format!(
                    "Setting {setting_name} cannot be empty for LDAP configuration to work",
                ))),
                None => Err(LdapError::MissingSettings(format!(
                    "Setting {setting_name} is required for LDAP configuration to work"
                ))),
            }
        }

        Ok(Self {
            ldap_member_attr: validate_string_setting(
                settings.ldap_member_attr,
                "ldap_member_attr",
            )?,
            ldap_group_member_attr: validate_string_setting(
                settings.ldap_group_member_attr,
                "ldap_group_member_attr",
            )?,
            ldap_groupname_attr: validate_string_setting(
                settings.ldap_groupname_attr,
                "ldap_groupname_attr",
            )?,
            ldap_username_attr: validate_string_setting(
                settings.ldap_username_attr,
                "ldap_username_attr",
            )?,
            ldap_group_obj_class: validate_string_setting(
                settings.ldap_group_obj_class,
                "ldap_group_obj_class",
            )?,
            ldap_user_obj_class: validate_string_setting(
                settings.ldap_user_obj_class,
                "ldap_user_obj_class",
            )?,
            ldap_user_search_base: validate_string_setting(
                settings.ldap_user_search_base,
                "ldap_user_search_base",
            )?,
            ldap_bind_username: validate_string_setting(
                settings.ldap_bind_username,
                "ldap_bind_username",
            )?,
            ldap_group_search_base: validate_string_setting(
                settings.ldap_group_search_base,
                "ldap_group_search_base",
            )?,
            ldap_user_auxiliary_obj_classes: settings.ldap_user_auxiliary_obj_classes,
            ldap_uses_ad: settings.ldap_uses_ad,
            ldap_user_rdn_attr: settings.ldap_user_rdn_attr,
            ldap_sync_groups: settings.ldap_sync_groups,
        })
    }
}

#[cfg(not(test))]
pub struct LDAPConnection {
    pub config: LDAPConfig,
    pub ldap: Ldap,
    pub url: String,
}

#[cfg(test)]
pub struct LDAPConnection {
    pub config: LDAPConfig,
    pub url: String,
}

impl LDAPConnection {
    /// Updates user state in LDAP based on the following rules:
    /// - If the user is disabled in Defguard, he will be removed from LDAP
    /// - If there are no sync groups defined or the user is in them but doesn't exist yet in LDAP, he will be added to LDAP and assigned to his groups
    /// - If the user is not in sync groups but is present in LDAP, he will be removed from LDAP
    ///
    /// Make sure to call this every time one of the above conditions changes (e.g. group addition, user disabling)
    pub(crate) async fn update_users_state(
        &mut self,
        users: Vec<&mut User<Id>>,
        pool: &PgPool,
    ) -> Result<(), LdapError> {
        debug!("Updating users state in LDAP");
        let transaction = pool.begin().await?;

        for user in users {
            let user_exists_in_ldap = self.user_exists(user).await?;
            let user_groups = user.member_of_names(pool).await?;
            let user_sync_allowed = user.ldap_sync_allowed(pool).await?;

            // User is disabled in Defguard or he is not in the defined sync groups
            // If they exist in LDAP, remove them
            if !user.is_active && user_exists_in_ldap {
                debug!("User {user} is disabled in Defguard, removing from LDAP");
                self.delete_user(user).await?;
                continue;
            }

            // No sync groups defined or user already belongs to them,
            // Add the user if they don't exist in LDAP already but are active in Defguard
            if user_sync_allowed && !user_exists_in_ldap {
                debug!("User {user} is not in LDAP, adding to LDAP");
                self.add_user(user, None, pool).await?;
                for group in user_groups {
                    self.add_user_to_group(user, &group).await?;
                }
                continue;
            }

            // We may bring user into the synchronization scope, sync his data (email, groups, etc.) based on
            // the authority
            if user_sync_allowed && user_exists_in_ldap {
                debug!(
                    "User {user} is in LDAP and is allowed to be synced, synchronizing his data"
                );
                self.sync_user_data(user, pool).await?;
                debug!("User {user} data synchronized");
                continue;
            }
        }

        transaction.commit().await?;

        Ok(())
    }

    /// Checks if user belongs to one of the defined sync groups in the LDAP server.
    async fn user_in_ldap_sync_groups<I>(&mut self, user: &User<I>) -> Result<bool, LdapError> {
        debug!("Checking if user {} is in LDAP sync groups", user.username);

        // Sync groups empty, we should sync all users
        if self.config.ldap_sync_groups.is_empty() {
            debug!("Sync groups were not defined, user {user} will be synced");
            return Ok(true);
        }

        let dn = self.config.user_dn(user.ldap_rdn_value());

        if !self.user_exists(user).await? {
            debug!("User {user} does not exist, not syncing user");
            return Ok(false);
        }

        let user_groups_entries = self.get_user_groups(&dn).await?;
        let user_groups_names = user_groups_entries
            .iter()
            .filter_map(|entry| {
                entry
                    .attrs
                    .get(&self.config.ldap_groupname_attr)
                    .and_then(|v| v.first())
            })
            .collect::<Vec<_>>();

        if user_groups_names
            .into_iter()
            .any(|group| self.config.ldap_sync_groups.contains(group))
        {
            debug!("User {user} is in sync groups, syncing user");
            Ok(true)
        } else {
            debug!("User {user} is not in sync groups, not syncing user");
            Ok(false)
        }
    }

    async fn group_exists(&mut self, groupname: &str) -> Result<bool, LdapError> {
        let groupname_attr = self.config.ldap_groupname_attr.clone();
        let res = self
            .search_groups(format!("({groupname_attr}={groupname})").as_str())
            .await?;

        Ok(!res.is_empty())
    }

    async fn user_exists_by_username(&mut self, username: &str) -> Result<bool, LdapError> {
        let username_attr = self.config.ldap_username_attr.clone();
        let res = self
            .search_users(format!("({username_attr}={username})").as_str())
            .await?;

        Ok(!res.is_empty())
    }

    async fn user_exists_by_rdn(&mut self, rdn: &str) -> Result<bool, LdapError> {
        let rdn_attr = self.config.get_rdn_attr();
        let res = self
            .search_users(format!("({rdn_attr}={rdn})").as_str())
            .await?;

        Ok(!res.is_empty())
    }

    async fn user_exists<I>(&mut self, user: &User<I>) -> Result<bool, LdapError> {
        let username = &user.username;
        let rdn = user.ldap_rdn_value();
        let username_exists = self.user_exists_by_username(username).await?;
        let rdn_exists = self.user_exists_by_rdn(rdn).await?;

        Ok(username_exists || rdn_exists)
    }

    // Checks if cn is available, including default LDAP admin class
    pub async fn is_username_available(&mut self, username: &str) -> bool {
        debug!("Checking if username {username} is available");
        let username_escape = ldap_escape(username);
        let users = self
            .search_users(&format!(
                "(&({}={username_escape})(|(objectClass={})))",
                self.config.ldap_username_attr, self.config.ldap_user_obj_class
            ))
            .await;
        match users {
            Ok(users) => users.is_empty(),
            _ => true,
        }
    }

    pub async fn is_rdn_available(&mut self, rdn: &str) -> bool {
        debug!("Checking if RDN {rdn} is available");
        let rdn_escape = ldap_escape(rdn);
        let users = self
            .search_users(&format!("({}={rdn_escape})", self.config.get_rdn_attr()))
            .await;
        match users {
            Ok(users) => users.is_empty(),
            _ => true,
        }
    }

    /// Retrieves user with given username from LDAP.
    pub async fn fetch_user_by_credentials(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<User, LdapError> {
        debug!("Performing LDAP user search: {username}");
        let username_escape = ldap_escape(username);
        let mut entries = self
            .search_users(&format!(
                "(&({}={username_escape})(objectClass={}))",
                self.config.ldap_username_attr, self.config.ldap_user_obj_class
            ))
            .await?;
        if entries.len() > 1 {
            return Err(LdapError::TooManyObjects);
        }
        if let Some(entry) = entries.pop() {
            info!("Performed LDAP user search: {username}");
            self.test_bind_user(&entry.dn, password).await?;
            User::from_searchentry(&entry, username, Some(password))
        } else {
            Err(LdapError::ObjectNotFound(format!(
                "User {username} not found",
            )))
        }
    }

    pub async fn get_user(&mut self, user: &User<Id>) -> Result<User, LdapError> {
        let rdn = user.ldap_rdn_value();
        debug!(
            "Trying to retrieve LDAP user with the following RDN: {}",
            rdn
        );
        let mut entries = self
            .search_users(&format!(
                "(&({}={rdn})(objectClass={}))",
                self.config.get_rdn_attr(),
                self.config.ldap_user_obj_class
            ))
            .await?;
        if entries.len() > 1 {
            return Err(LdapError::TooManyObjects);
        }
        if let Some(entry) = entries.pop() {
            info!("Performed LDAP user search: {rdn}");
            User::from_searchentry(&entry, &user.username, None)
        } else {
            Err(LdapError::ObjectNotFound(format!("User {rdn} not found",)))
        }
    }

    /// Adds user to LDAP.
    pub async fn add_user(
        &mut self,
        user: &mut User<Id>,
        password: Option<&str>,
        pool: &PgPool,
    ) -> Result<(), LdapError> {
        debug!("Adding LDAP user {}", user.username);
        let user_rdn = user.ldap_rdn_value();
        let dn = self.config.user_dn(user_rdn);
        let password_is_random = password.is_none();
        let password = if let Some(password) = password {
            debug!("Using provided password for user {}", user.username);
            password.to_string()
        } else {
            // ldap may not accept no password, this is a workaround when we don't have access to the
            // user's password
            debug!(
                "Generating random password for user {}, as no password has been specified",
                user.username
            );
            let random_password = rand::thread_rng()
                .sample_iter(&rand::distributions::Alphanumeric)
                .take(32)
                .map(char::from)
                .collect::<String>();

            debug!("Generated random password for user {}", user.username);
            random_password
        };
        let ssha_password = hash::salted_sha1_hash(&password);
        let nt_password = hash::nthash(&password);
        let user_obj_classes = self.config.get_all_user_obj_classes();
        let username_attr = self.config.ldap_username_attr.clone();
        let rdn_attr = self.config.get_rdn_attr().to_string();
        if !self.is_username_available(&user.username).await
            || !self.is_rdn_available(user_rdn).await
        {
            return Err(LdapError::ObjectAlreadyExists(format!(
                "User with username {} or RDN {user_rdn} already exists",
                user.username
            )));
        }
        self.add(
            &dn,
            user.as_ldap_attrs(
                &ssha_password,
                &nt_password,
                user_obj_classes.iter().map(|s| s.as_str()).collect(),
                self.config.ldap_uses_ad,
                &username_attr,
                &rdn_attr,
            ),
        )
        .await?;
        if self.config.ldap_uses_ad {
            self.set_password(user, &password).await?;
            self.activate_ad_user(user_rdn).await?;
        }
        if password_is_random {
            user.ldap_pass_randomized = true;
            user.save(pool).await?;
        }
        info!("Added LDAP user {}", user.username);
        Ok(())
    }

    /// Modifies existing LDAP user.
    pub async fn modify_user(
        &mut self,
        old_username: &str,
        user: &mut User<Id>,
        pool: &PgPool,
    ) -> Result<(), LdapError> {
        debug!("Modifying user {old_username} in LDAP");
        // If we're using the username as the RDN, also update the RDN value on user if his username has been changed
        let old_rdn = if self.config.using_username_as_rdn() {
            user.ldap_rdn = Some(user.username.clone());
            old_username
        } else {
            user.ldap_rdn_value()
        };
        if !self.user_exists_by_rdn(old_rdn).await? {
            return Err(LdapError::ObjectNotFound(format!(
                "User {old_username} not found in LDAP, cannot modify",
            )));
        }
        let old_dn = self.config.user_dn(old_rdn);
        let new_dn = self.config.user_dn(user.ldap_rdn_value());
        let config = self.config.clone();
        let mods = user.as_ldap_mod(&config);
        self.modify(&old_dn, &new_dn, mods).await?;
        // Commit only now, after we actually sent the changes to LDAP
        user.save(pool).await?;
        info!("Modified user {old_username} in LDAP");

        Ok(())
    }

    fn group_entry_to_name(&self, entry: SearchEntry) -> Result<String, LdapError> {
        entry
            .attrs
            .get(&self.config.ldap_groupname_attr)
            .and_then(|v| v.first())
            .map(|name| name.to_string())
            .ok_or_else(|| {
                LdapError::ObjectNotFound(format!(
                    "Couldn't extract a group name from searchentry {:?}.",
                    entry
                ))
            })
    }

    /// Deletes user from LDAP.
    pub async fn delete_user<I>(&mut self, user: &User<I>) -> Result<(), LdapError> {
        let user_rdn_value = user.ldap_rdn_value();
        debug!("Deleting user {user}");
        let dn = self.config.user_dn(user_rdn_value);
        debug!("Removing group memberships first...");
        let user_groups = self.get_user_groups(&dn).await?;
        debug!("Removing user from groups: {user_groups:?}");
        for group in user_groups {
            debug!("Removing user from group {group:?}");
            match self.group_entry_to_name(group) {
                Ok(groupname) => {
                    self.remove_user_from_group(user, &groupname).await?;
                    debug!("Removed user from group {groupname}");
                }
                Err(e) => {
                    warn!("Failed to remove user from group: {e}");
                }
            }
        }
        self.delete(&dn).await?;
        info!("Deleted user {user}");

        Ok(())
    }

    pub async fn activate_ad_user(&mut self, user_rdn_value: &str) -> Result<(), LdapError> {
        debug!("Activating user {user_rdn_value}");
        let user_dn = self.config.user_dn(user_rdn_value);
        self.modify(
            &user_dn,
            &user_dn,
            vec![
                // Enables the user
                Mod::Replace("userAccountControl", hashset!["512"]),
                // The user doesn't have to change password at next login
                Mod::Replace("pwdLastSet", hashset!["-1"]),
            ],
        )
        .await?;
        info!("Activated user {user_rdn_value}");

        Ok(())
    }

    /// Changes user password.
    pub async fn set_password<I>(
        &mut self,
        user: &User<I>,
        password: &str,
    ) -> Result<(), LdapError> {
        debug!("Setting password for user {user}");
        let user_dn = self.config.user_dn(user.ldap_rdn_value());

        if self.config.ldap_uses_ad {
            let unicode_pwd = hash::unicode_pwd(password);
            // We need to deal with bytes here as both the attribute and value are expected to be in
            // binary
            let mods = vec![Mod::Replace(
                "unicodePwd".as_bytes(),
                hashset![unicode_pwd.as_ref()],
            )];
            self.modify(&user_dn, &user_dn, mods).await?;
            info!("Password set for user {user}");
        } else {
            let ssha_password = hash::salted_sha1_hash(password);
            let nt_password = hash::nthash(password);
            let mut mods = Vec::new();
            if self
                .config
                .ldap_user_auxiliary_obj_classes
                .contains(&UserObjectClass::SimpleSecurityObject.into())
            {
                mods.push(Mod::Replace(
                    "userPassword",
                    hashset![ssha_password.as_str()],
                ));
            }

            if self
                .config
                .ldap_user_auxiliary_obj_classes
                .contains(&UserObjectClass::SambaSamAccount.into())
            {
                mods.push(Mod::Replace(
                    "sambaNTPassword",
                    hashset![nt_password.as_str()],
                ));
            }

            if mods.is_empty() {
                return Err(LdapError::MissingSettings(
                    format!("Can't set password as no password object class has been defined for the user {user}."),
                ));
            }

            self.modify(&user_dn, &user_dn, mods).await?;
            info!("Password set for user {user}");
        };

        Ok(())
    }

    /// This exists as some LDAP servers don't allow for creating empty groups
    /// Notable example: OpenLDAP
    pub async fn add_group_with_members<I>(
        &mut self,
        group_name: &str,
        members: Vec<&User<I>>,
    ) -> Result<(), LdapError> {
        debug!("Adding LDAP group {}", group_name);
        let dn = self.config.group_dn(group_name);
        let group_obj_class = self.config.ldap_group_obj_class.clone();
        let groupname_attr = self.config.ldap_groupname_attr.clone();
        let mut group_attrs = vec![
            ("objectClass", hashset![group_obj_class.as_str()]),
            (groupname_attr.as_str(), hashset![group_name]),
        ];
        //   extent the group attr with multiple members
        let member_dns = members
            .into_iter()
            .map(|member| self.config.user_dn(member.ldap_rdn_value()))
            .collect::<Vec<_>>();
        let member_group_attr = self.config.ldap_group_member_attr.clone();
        let member_refs: HashSet<&str> = member_dns.iter().map(|s| s.as_str()).collect();

        for member_ref in member_refs {
            group_attrs.push((member_group_attr.as_str(), hashset![member_ref]));
        }

        self.add(&dn, group_attrs).await?;
        info!(
            "Added LDAP group {} with members {}",
            group_name,
            member_dns
                .iter()
                .map(|dn| dn.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );

        Ok(())
    }

    /// Modifies LDAP group.
    pub async fn modify_group(
        &mut self,
        groupname: &str,
        group: &db::Group<Id>,
    ) -> Result<(), LdapError> {
        debug!("Modifying LDAP group {groupname}");
        let old_dn = self.config.group_dn(groupname);
        let new_dn = self.config.group_dn(&group.name);
        let groupname_attr = self.config.ldap_groupname_attr.clone();
        self.modify(
            &old_dn,
            &new_dn,
            vec![Mod::Replace(
                groupname_attr.as_str(),
                hashset![group.name.as_str()],
            )],
        )
        .await?;
        info!("Modified LDAP group {groupname}");

        Ok(())
    }

    pub async fn delete_group(&mut self, groupname: &str) -> Result<(), LdapError> {
        debug!("Deleting LDAP group {groupname}");
        let dn = self.config.group_dn(groupname);
        self.delete(&dn).await?;
        info!("Deleted LDAP group {groupname}");

        Ok(())
    }

    /// Add user to a group.
    pub async fn add_user_to_group<I>(
        &mut self,
        user: &User<I>,
        groupname: &str,
    ) -> Result<(), LdapError> {
        debug!(
            "Adding user {user} to group {groupname} in LDAP, checking if that group exists first..."
        );
        let user_dn = self.config.user_dn(user.ldap_rdn_value());
        if self.is_member_of(&user_dn, groupname).await? {
            debug!("User {user} is already a member of group {groupname}, skipping");
            return Ok(());
        }
        if !self.group_exists(groupname).await? {
            debug!("Group {groupname} doesn't exist in LDAP, creating it");
            self.add_group_with_members(groupname, vec![user]).await?;
            debug!("Group {groupname} created and member added in LDAP");
        } else {
            debug!("Group {groupname} exists in LDAP, adding user {user} to it");
            let group_dn = self.config.group_dn(groupname);
            self.modify(
                &group_dn,
                &group_dn,
                vec![Mod::Add(
                    &self.config.ldap_group_member_attr.clone(),
                    hashset![&user_dn],
                )],
            )
            .await?;
            debug!("Added user {user} to group {groupname} in LDAP");
        }
        info!("Added user {user} to group {groupname} in LDAP");
        Ok(())
    }

    /// Remove user from a group.
    pub async fn remove_user_from_group<I>(
        &mut self,
        user: &User<I>,
        groupname: &str,
    ) -> Result<(), LdapError> {
        debug!("Removing user {user} from group {groupname} in LDAP");
        let user_dn = self.config.user_dn(user.ldap_rdn_value());
        if !self.is_member_of(&user_dn, groupname).await? {
            debug!("User {user} is not a member of group {groupname}, skipping");
            return Ok(());
        }
        let members = self.get_group_members(groupname).await?;
        if members.len() > 1 {
            let group_dn = self.config.group_dn(groupname);
            self.modify(
                &group_dn,
                &group_dn,
                vec![Mod::Delete(
                    &self.config.ldap_group_member_attr.clone(),
                    hashset![&user_dn],
                )],
            )
            .await?;
            debug!("Removed user {user} from group {groupname} in LDAP");
        } else {
            debug!("Group {groupname} has only one member, removing the whole group",);
            self.delete_group(groupname).await?;
            debug!("Removed group {groupname} from LDAP");
        }

        info!("Removed user {user} from group {groupname} in LDAP");

        Ok(())
    }
}
