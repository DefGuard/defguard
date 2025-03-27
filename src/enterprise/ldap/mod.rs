use std::{collections::HashSet, future::Future, time::Duration};

use ldap3::{drive, ldap_escape, Ldap, LdapConnAsync, LdapConnSettings, Mod, Scope, SearchEntry};
use model::UserObjectClass;
use rand::Rng;
use sqlx::PgPool;
use sync::{get_ldap_sync_status, is_ldap_desynced, set_ldap_sync_status, SyncStatus};

use self::error::LdapError;
use crate::{
    db::{self, models::settings::update_current_settings, Id, Settings, User},
    enterprise::{is_enterprise_enabled, limits::update_counts},
};

pub mod error;
pub mod hash;
pub mod model;
pub mod sync;
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
}

impl LDAPConfig {
    /// Constructs user distinguished name.
    #[must_use]
    pub(crate) fn user_dn(&self, username: &str) -> String {
        format!(
            "{}={username},{}",
            self.ldap_username_attr, self.ldap_user_search_base,
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
        })
    }
}

pub struct LDAPConnection {
    pub config: LDAPConfig,
    pub ldap: Ldap,
    pub url: String,
}

impl LDAPConnection {
    pub async fn create() -> Result<LDAPConnection, LdapError> {
        let settings = Settings::get_current_settings();
        let config = LDAPConfig::try_from(settings.clone())?;
        let url = settings.ldap_url.ok_or(LdapError::MissingSettings(
            "LDAP URL is required for LDAP configuration to work".to_string(),
        ))?;
        let password = settings
            .ldap_bind_password
            .ok_or(LdapError::MissingSettings("LDAP bind password".to_string()))?;
        let conn_settings = LdapConnSettings::new()
            .set_starttls(settings.ldap_use_starttls)
            .set_no_tls_verify(!settings.ldap_tls_verify_cert)
            .set_conn_timeout(Duration::from_secs(8));
        let (conn, mut ldap) = LdapConnAsync::with_settings(conn_settings, &url).await?;
        drive!(conn);
        info!("Connected to LDAP: {url}");
        ldap.simple_bind(&config.ldap_bind_username, password.expose_secret())
            .await?
            .success()?;

        Ok(Self { config, ldap, url })
    }

    /// Searches LDAP for users.
    pub async fn search_users(&mut self, filter: &str) -> Result<Vec<SearchEntry>, LdapError> {
        let (rs, res) = self
            .ldap
            .search(
                &self.config.ldap_user_search_base,
                Scope::Subtree,
                filter,
                vec!["*", &self.config.ldap_member_attr],
            )
            .await?
            .success()?;
        debug!("LDAP user search result: {res:?}");
        debug!("Performed LDAP user search with filter = {filter}");

        Ok(rs.into_iter().map(SearchEntry::construct).collect())
    }

    async fn test_bind_user(&self, dn: &str, password: &str) -> Result<(), LdapError> {
        debug!("Testing LDAP bind for user {dn}");
        let settings = Settings::get_current_settings();
        let conn_settings = LdapConnSettings::new()
            .set_starttls(settings.ldap_use_starttls)
            .set_no_tls_verify(!settings.ldap_tls_verify_cert)
            .set_conn_timeout(Duration::from_secs(8));
        let (conn, mut ldap) = LdapConnAsync::with_settings(conn_settings, &self.url).await?;
        drive!(conn);
        let res = ldap.simple_bind(dn, password).await?.success()?;
        debug!("LDAP user bind test result: {res}");
        ldap.unbind().await?;
        info!("LDAP bind test for user {dn} successful");
        Ok(())
    }

    // Check what groups user is member of
    pub async fn get_user_groups(&mut self, user_dn: &str) -> Result<Vec<SearchEntry>, LdapError> {
        let filter = format!("({}={})", self.config.ldap_group_member_attr, user_dn);
        let (rs, res) = self
            .ldap
            .search(
                &self.config.ldap_group_search_base,
                Scope::Subtree,
                filter.as_str(),
                vec![&self.config.ldap_groupname_attr],
            )
            .await?
            .success()?;
        debug!("LDAP user groups search result: {res}");
        debug!("Performed LDAP group search with filter = {filter}");
        Ok(rs.into_iter().map(SearchEntry::construct).collect())
    }

    async fn group_exists(&mut self, groupname: &str) -> Result<bool, LdapError> {
        let groupname_attr = self.config.ldap_groupname_attr.clone();
        let res = self
            .search_groups(format!("({groupname_attr}={groupname})").as_str())
            .await?;

        Ok(!res.is_empty())
    }

    /// Searches LDAP for groups.
    async fn search_groups(&mut self, filter: &str) -> Result<Vec<SearchEntry>, LdapError> {
        let (rs, res) = self
            .ldap
            .search(
                &self.config.ldap_group_search_base,
                Scope::Subtree,
                filter,
                vec![
                    &self.config.ldap_username_attr,
                    &self.config.ldap_group_member_attr,
                ],
            )
            .await?
            .success()?;
        debug!("LDAP group search result: {res}");
        info!("Performed LDAP group search with filter = {filter}");
        Ok(rs.into_iter().map(SearchEntry::construct).collect())
    }

    /// Creates LDAP object with specified distinguished name and attributes.
    async fn add(&mut self, dn: &str, attrs: Vec<(&str, HashSet<&str>)>) -> Result<(), LdapError> {
        debug!("Adding object {dn}");
        let result = self.ldap.add(dn, attrs).await?.success()?;
        debug!("LDAP add result: {result:?}");
        info!("Added object {dn}");

        Ok(())
    }

    /// Updates LDAP object with specified distinguished name and attributes.
    async fn modify(
        &mut self,
        old_dn: &str,
        new_dn: &str,
        mods: Vec<Mod<&str>>,
    ) -> Result<(), LdapError> {
        debug!("Modifying LDAP object {old_dn}");
        let result = self.ldap.modify(old_dn, mods).await?;
        debug!("LDAP modification result: {result:?}");
        if old_dn != new_dn {
            debug!("Renaming LDAP object {old_dn} to {new_dn}");
            if let Some((new_rdn, _rest)) = new_dn.split_once(',') {
                let result = self.ldap.modifydn(old_dn, new_rdn, true, None).await?;
                debug!("LDAP rename result: {result:?}");
            } else {
                warn!("Failed to rename LDAP object {old_dn} to {new_dn}, new DN is invalid");
            }
        }
        info!("Modified LDAP object {old_dn}");

        Ok(())
    }

    /// Deletes LDAP object with specified distinguished name.
    pub async fn delete(&mut self, dn: &str) -> Result<(), LdapError> {
        debug!("Deleting LDAP object {dn}");
        let result = self.ldap.delete(dn).await?;
        debug!("LDAP deletion result: {result:?}");
        info!("Deleted LDAP object {dn}");

        Ok(())
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

    /// Retrieves user with given username from LDAP.
    pub async fn get_user(&mut self, username: &str, password: &str) -> Result<User, LdapError> {
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

    /// Adds user to LDAP.
    pub async fn add_user(
        &mut self,
        user: &mut User<Id>,
        password: Option<&str>,
        pool: &PgPool,
    ) -> Result<(), LdapError> {
        debug!("Adding LDAP user {}", user.username);
        let dn = self.config.user_dn(&user.username);
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
        self.add(
            &dn,
            user.as_ldap_attrs(
                &ssha_password,
                &nt_password,
                user_obj_classes.iter().map(|s| s.as_str()).collect(),
                self.config.ldap_uses_ad,
            ),
        )
        .await?;
        if self.config.ldap_uses_ad {
            self.set_password(&user.username, &password).await?;
            self.activate_ad_user(&user.username).await?;
        }
        if password_is_random {
            user.ldap_pass_randomized = true;
            user.save(pool).await?;
        }
        info!("Added LDAP user {}", user.username);
        Ok(())
    }

    /// Modifies LDAP user.
    pub async fn modify_user(&mut self, username: &str, user: &User<Id>) -> Result<(), LdapError> {
        debug!("Modifying user {username}");
        let old_dn = self.config.user_dn(username);
        let new_dn = self.config.user_dn(&user.username);
        self.modify(&old_dn, &new_dn, user.as_ldap_mod(&self.config))
            .await?;
        info!("Modified user {username}");

        Ok(())
    }

    /// Deletes user from LDAP.
    pub async fn delete_user(&mut self, username: &str) -> Result<(), LdapError> {
        debug!("Deleting user {username}");
        let dn = self.config.user_dn(username);
        debug!("Removing group memberships first...");
        let user_groups = self.get_user_groups(&dn).await?;
        debug!("Removing user from groups: {user_groups:?}");
        for group in user_groups {
            debug!("Removing user from group {group:?}");
            if let Some(groupname) = group
                .attrs
                .get(&self.config.ldap_groupname_attr)
                .and_then(|v| v.first())
            {
                self.remove_user_from_group(username, groupname).await?;
                debug!("Removed user from group {groupname}");
            } else {
                warn!("Group without name found for user {username}, full group entry: {group:?}");
            }
        }
        self.delete(&dn).await?;
        info!("Deleted user {username}");

        Ok(())
    }

    pub async fn set_user_status(&mut self, username: &str, active: bool) -> Result<(), LdapError> {
        debug!("Setting user {username} status to {active}");
        let user_dn = self.config.user_dn(username);
        let user_account_control = if active { "512" } else { "514" };
        self.modify(
            &user_dn,
            &user_dn,
            vec![Mod::Replace(
                "userAccountControl",
                hashset![user_account_control],
            )],
        )
        .await?;
        debug!("Set user {username} status to {active}");

        Ok(())
    }

    pub async fn activate_ad_user(&mut self, username: &str) -> Result<(), LdapError> {
        debug!("Activating user {username}");
        let user_dn = self.config.user_dn(username);
        self.modify(
            &user_dn,
            &user_dn,
            vec![
                Mod::Replace("userAccountControl", hashset!["512"]),
                Mod::Replace("pwdLastSet", hashset!["-1"]),
            ],
        )
        .await?;
        info!("Activated user {username}");

        Ok(())
    }

    /// Changes user password.
    pub async fn set_password(&mut self, username: &str, password: &str) -> Result<(), LdapError> {
        debug!("Setting password for user {username}");
        let user_dn = self.config.user_dn(username);

        if self.config.ldap_uses_ad {
            let unicode_pwd = hash::unicode_pwd(password);
            // We need to deal with bytes here as both the attribute and value are expected to be in
            // binary
            let mods = vec![Mod::Replace(
                "unicodePwd".as_bytes(),
                hashset![unicode_pwd.as_ref()],
            )];
            let out = self.ldap.modify(&user_dn, mods).await?;
            debug!("LDAP modification result: {out:?}");
            info!("Password set for user {username}");
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
                    format!("Can't set password as no password object class has been defined for the user {username}."),
                ));
            }

            self.modify(&user_dn, &user_dn, mods).await?;
            info!("Password set for user {username}");
        };

        Ok(())
    }

    pub async fn add_group_with_members(
        &mut self,
        group_name: &str,
        members: Vec<&str>,
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
            .map(|member| self.config.user_dn(member))
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
    pub async fn add_user_to_group(
        &mut self,
        username: &str,
        groupname: &str,
    ) -> Result<(), LdapError> {
        debug!("Adding user {username} to group {groupname} in LDAP");
        let user_dn = self.config.user_dn(username);
        let group_dn = self.config.group_dn(groupname);
        self.modify(
            &group_dn,
            &group_dn,
            vec![Mod::Add(
                &self.config.ldap_group_member_attr.clone(),
                hashset![user_dn.as_str()],
            )],
        )
        .await?;

        info!("Added user {username} to group {groupname} in LDAP");
        Ok(())
    }

    /// Remove user from a group.
    pub async fn remove_user_from_group(
        &mut self,
        username: &str,
        groupname: &str,
    ) -> Result<(), LdapError> {
        debug!("Removing user {username} from group {groupname} in LDAP");
        let members = self.get_group_members(groupname).await?;
        if members.len() > 1 {
            let user_dn = self.config.user_dn(username);
            let group_dn = self.config.group_dn(groupname);
            self.modify(
                &group_dn,
                &group_dn,
                vec![Mod::Delete(
                    &self.config.ldap_group_member_attr.clone(),
                    hashset![user_dn.as_str()],
                )],
            )
            .await?;
            debug!("Removed user {username} from group {groupname} in LDAP");
        } else {
            debug!("Group {groupname} has only one member, removing the whole group",);
            self.delete_group(groupname).await?;
            debug!("Removed group {groupname} from LDAP");
        }

        info!("Removed user {username} from group {groupname} in LDAP");

        Ok(())
    }
}
