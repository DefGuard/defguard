use std::collections::HashSet;

use ldap3::{drive, ldap_escape, Ldap, LdapConnAsync, Mod, Scope, SearchEntry};

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
}

impl LDAPConfig {
    /// Constructs user distinguished name.
    #[must_use]
    pub fn user_dn(&self, username: &str) -> String {
        format!(
            "{}={username},{}",
            self.ldap_username_attr, self.ldap_user_search_base,
        )
    }

    /// Constructs group distinguished name.
    #[must_use]
    pub fn group_dn(&self, groupname: &str) -> String {
        format!(
            "{}={groupname},{}",
            self.ldap_groupname_attr, self.ldap_group_search_base,
        )
    }
}

impl TryFrom<Settings> for LDAPConfig {
    type Error = LdapError;

    fn try_from(settings: Settings) -> Result<LDAPConfig, LdapError> {
        Ok(Self {
            ldap_member_attr: settings
                .ldap_member_attr
                .ok_or(LdapError::MissingSettings)?,
            ldap_group_member_attr: settings
                .ldap_group_member_attr
                .ok_or(LdapError::MissingSettings)?,
            ldap_groupname_attr: settings
                .ldap_groupname_attr
                .ok_or(LdapError::MissingSettings)?,
            ldap_username_attr: settings
                .ldap_username_attr
                .ok_or(LdapError::MissingSettings)?,
            ldap_group_obj_class: settings
                .ldap_group_obj_class
                .ok_or(LdapError::MissingSettings)?,
            ldap_user_obj_class: settings
                .ldap_user_obj_class
                .ok_or(LdapError::MissingSettings)?,
            ldap_user_search_base: settings
                .ldap_user_search_base
                .ok_or(LdapError::MissingSettings)?,
            ldap_bind_username: settings
                .ldap_bind_username
                .ok_or(LdapError::MissingSettings)?,
            ldap_group_search_base: settings
                .ldap_group_search_base
                .ok_or(LdapError::MissingSettings)?,
        })
    }
}

pub struct LDAPConnection {
    config: LDAPConfig,
    ldap: Ldap,
    url: String,
}

impl LDAPConnection {
    pub async fn create() -> Result<LDAPConnection, LdapError> {
        let settings = Settings::get_current_settings();
        let config = LDAPConfig::try_from(settings.clone())?;
        let url = settings.ldap_url.ok_or(LdapError::MissingSettings)?;
        let password = settings
            .ldap_bind_password
            .ok_or(LdapError::MissingSettings)?;
        let (conn, mut ldap) = LdapConnAsync::new(&url).await?;
        drive!(conn);
        info!("Connected to LDAP: {url}");
        ldap.simple_bind(&config.ldap_bind_username, password.expose_secret())
            .await?
            .success()?;

        Ok(Self { config, ldap, url })
    }

    /// Searches LDAP for users.
    async fn search_users(&mut self, filter: &str) -> Result<Vec<SearchEntry>, LdapError> {
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
        let (conn, mut ldap) = LdapConnAsync::new(&self.url).await?;
        drive!(conn);
        ldap.simple_bind(dn, password).await?.success()?;
        ldap.unbind().await?;
        Ok(())
    }

    // /// Searches LDAP for groups.
    // async fn search_groups(&mut self, filter: &str) -> Result<Vec<SearchEntry>, LdapError> {
    //     let (rs, _res) = self
    //         .ldap
    //         .search(
    //             &self.config.ldap_group_search_base,
    //             Scope::Subtree,
    //             filter,
    //             vec![
    //                 &self.config.ldap_username_attr,
    //                 &self.config.ldap_group_member_attr,
    //             ],
    //         )
    //         .await?
    //         .success()?;
    //     info!("Performed LDAP group search with filter = {filter}");
    //     Ok(rs.into_iter().map(SearchEntry::construct).collect())
    // }

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
            Ok(User::from_searchentry(&entry, username, password))
        } else {
            Err(LdapError::ObjectNotFound(format!(
                "User {username} not found",
            )))
        }
    }

    /// Adds user to LDAP.
    pub async fn add_user(&mut self, user: &User<Id>, password: &str) -> Result<(), LdapError> {
        debug!("Adding LDAP user {}", user.username);
        let dn = self.config.user_dn(&user.username);
        let ssha_password = hash::salted_sha1_hash(password);
        let ht_password = hash::nthash(password);
        self.add(&dn, user.as_ldap_attrs(&ssha_password, &ht_password))
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
        group: &db::Group,
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
