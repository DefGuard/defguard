use std::collections::HashSet;

use ldap3::{drive, ldap_escape, Ldap, LdapConnAsync, LdapConnSettings, Mod, Scope, SearchEntry};
use rand::Rng;

use self::error::LdapError;
use crate::db::{self, Id, Settings, User};

pub mod error;
pub mod hash;
pub mod model;
pub mod utils;

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
    pub ldap_samba_enabled: bool,
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
}

impl TryFrom<Settings> for LDAPConfig {
    type Error = LdapError;

    fn try_from(settings: Settings) -> Result<LDAPConfig, LdapError> {
        // Helper function to validate non-empty string settings
        fn validate_setting(
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
            ldap_member_attr: validate_setting(settings.ldap_member_attr, "ldap_member_attr")?,
            ldap_group_member_attr: validate_setting(
                settings.ldap_group_member_attr,
                "ldap_group_member_attr",
            )?,
            ldap_groupname_attr: validate_setting(
                settings.ldap_groupname_attr,
                "ldap_groupname_attr",
            )?,
            ldap_username_attr: validate_setting(
                settings.ldap_username_attr,
                "ldap_username_attr",
            )?,
            ldap_group_obj_class: validate_setting(
                settings.ldap_group_obj_class,
                "ldap_group_obj_class",
            )?,
            ldap_user_obj_class: validate_setting(
                settings.ldap_user_obj_class,
                "ldap_user_obj_class",
            )?,
            ldap_user_search_base: validate_setting(
                settings.ldap_user_search_base,
                "ldap_user_search_base",
            )?,
            ldap_bind_username: validate_setting(
                settings.ldap_bind_username,
                "ldap_bind_username",
            )?,
            ldap_group_search_base: validate_setting(
                settings.ldap_group_search_base,
                "ldap_group_search_base",
            )?,
            ldap_samba_enabled: settings.ldap_samba_enabled,
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
            .set_no_tls_verify(!settings.ldap_tls_verify_cert);
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
        let (rs, _res) = self
            .ldap
            .search(
                &self.config.ldap_user_search_base,
                Scope::Subtree,
                filter,
                vec!["*", &self.config.ldap_member_attr],
            )
            .await?
            .success()?;
        info!("Performed LDAP user search with filter = {filter}");

        Ok(rs.into_iter().map(SearchEntry::construct).collect())
    }

    async fn test_bind_user(&self, dn: &str, password: &str) -> Result<(), LdapError> {
        let settings = Settings::get_current_settings();
        let conn_settings = LdapConnSettings::new()
            .set_starttls(settings.ldap_use_starttls)
            .set_no_tls_verify(!settings.ldap_tls_verify_cert);
        let (conn, mut ldap) = LdapConnAsync::with_settings(conn_settings, &self.url).await?;
        drive!(conn);
        ldap.simple_bind(dn, password).await?.success()?;
        ldap.unbind().await?;
        Ok(())
    }

    // Check what groups user is member of
    pub async fn get_user_groups(&mut self, user_dn: &str) -> Result<Vec<SearchEntry>, LdapError> {
        let filter = format!("({}={})", self.config.ldap_group_member_attr, user_dn);
        let (rs, _res) = self
            .ldap
            .search(
                &self.config.ldap_group_search_base,
                Scope::Subtree,
                filter.as_str(),
                vec![&self.config.ldap_groupname_attr],
            )
            .await?
            .success()?;
        debug!("Performed LDAP group search with filter = {filter}");
        Ok(rs.into_iter().map(SearchEntry::construct).collect())
    }

    async fn group_exists(&mut self, groupname: &str) -> Result<bool, LdapError> {
        let res = self
            .search_groups(format!("(cn={groupname})").as_str())
            .await?;

        Ok(!res.is_empty())
    }

    /// Searches LDAP for groups.
    async fn search_groups(&mut self, filter: &str) -> Result<Vec<SearchEntry>, LdapError> {
        let (rs, _res) = self
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
        info!("Performed LDAP group search with filter = {filter}");
        Ok(rs.into_iter().map(SearchEntry::construct).collect())
    }

    /// Creates LDAP object with specified distinguished name and attributes.
    async fn add(&mut self, dn: &str, attrs: Vec<(&str, HashSet<&str>)>) -> Result<(), LdapError> {
        debug!("Adding object {dn}");
        self.ldap.add(dn, attrs).await?.success()?;
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
        self.ldap.modify(old_dn, mods).await?;
        if old_dn != new_dn {
            if let Some((new_rdn, _rest)) = new_dn.split_once(',') {
                self.ldap.modifydn(old_dn, new_rdn, true, None).await?;
            }
        }
        info!("Modified LDAP object {old_dn}");

        Ok(())
    }

    /// Deletes LDAP object with specified distinguished name.
    pub async fn delete(&mut self, dn: &str) -> Result<(), LdapError> {
        debug!("Deleting LDAP object {dn}");
        self.ldap.delete(dn).await?;
        info!("Deleted LDAP object {dn}");

        Ok(())
    }

    // Checks if cn is available, including default LDAP admin class
    pub async fn is_username_available(&mut self, username: &str) -> bool {
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
        user: &User<Id>,
        password: Option<&str>,
    ) -> Result<(), LdapError> {
        debug!("Adding LDAP user {}", user.username);
        let dn = self.config.user_dn(&user.username);
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
        self.add(&dn, user.as_ldap_attrs(&ssha_password, &nt_password))
            .await?;
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

    /// Changes user password.
    pub async fn set_password(&mut self, username: &str, password: &str) -> Result<(), LdapError> {
        debug!("Setting password for user {username}");
        let user_dn = self.config.user_dn(username);
        let ssha_password = hash::salted_sha1_hash(password);
        let nt_password = hash::nthash(password);
        self.modify(
            &user_dn,
            &user_dn,
            vec![
                Mod::Replace("userPassword", hashset![ssha_password.as_str()]),
                Mod::Replace("sambaNTPassword", hashset![nt_password.as_str()]),
            ],
        )
        .await?;
        info!("Password set for user {username}");

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
        let mut group_attrs = vec![
            ("objectClass", hashset![group_obj_class.as_str()]),
            ("cn", hashset![group_name]),
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

    // /// Retrieves group with given groupname from LDAP.
    // pub async fn get_group(&mut self, groupname: &str) -> Result<Group, LdapError> {
    //     debug!("Performing LDAP group search: {groupname}");
    //     let mut enties = self
    //         .search_groups(&format!(
    //             "(&({}={})(objectClass={}))",
    //             self.config.ldap_groupname_attr, groupname, self.config.ldap_group_obj_class
    //         ))
    //         .await?;
    //     if let Some(entry) = enties.pop() {
    //         info!("Performed LDAP user search: {groupname}");
    //         Ok(Group::from_searchentry(&entry, &self.config))
    //     } else {
    //         Err(LdapError::ObjectNotFound(format!(
    //             "Group {groupname} not found"
    //         )))
    //     }
    // }

    /// Modifies LDAP group.
    pub async fn modify_group(
        &mut self,
        groupname: &str,
        group: &db::Group<Id>,
    ) -> Result<(), LdapError> {
        debug!("Modifying LDAP group {groupname}");
        let old_dn = self.config.group_dn(groupname);
        let new_dn = self.config.group_dn(&group.name);
        self.modify(
            &old_dn,
            &new_dn,
            vec![Mod::Replace("cn", hashset![group.name.as_str()])],
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

    // /// Lists groups satisfying specified criteria
    // pub async fn get_groups(&mut self) -> Result<Vec<Group>, LdapError> {
    //     debug!("Performing LDAP group search");
    //     let mut entries = self
    //         .search_groups(&format!(
    //             "(objectClass={})",
    //             self.config.ldap_group_obj_class
    //         ))
    //         .await?;
    //     let users = entries
    //         .drain(..)
    //         .map(|entry| Group::from_searchentry(&entry, &self.config))
    //         .collect();
    //     info!("Performed LDAP group search");
    //     Ok(users)
    // }

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

        Ok(())
    }

    /// Remove user from a group.
    pub async fn remove_user_from_group(
        &mut self,
        username: &str,
        groupname: &str,
    ) -> Result<(), LdapError> {
        debug!("Removing user {username} from group {groupname} in LDAP");
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

        Ok(())
    }
}
