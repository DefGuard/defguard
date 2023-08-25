use crate::{config::DefGuardConfig, db::User};
use error::OriLDAPError;
use ldap3::{drive, Ldap, LdapConnAsync, Mod, Scope, SearchEntry};
use model::Group;
use secrecy::ExposeSecret;
use std::collections::HashSet;

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

pub struct LDAPConnection<'a> {
    config: &'a DefGuardConfig,
    ldap: Ldap,
}

impl<'a> LDAPConnection<'a> {
    pub async fn create(config: &'a DefGuardConfig) -> Result<LDAPConnection<'a>, OriLDAPError> {
        let (conn, mut ldap) = LdapConnAsync::new(&config.ldap_url).await?;
        drive!(conn);
        info!("Connected to LDAP: {}", &config.ldap_url);
        ldap.simple_bind(
            &config.ldap_bind_username,
            config.ldap_bind_password.expose_secret(),
        )
        .await?
        .success()?;
        Ok(Self { config, ldap })
    }

    /// Searches LDAP for users.
    async fn search_users(&mut self, filter: &str) -> Result<Vec<SearchEntry>, OriLDAPError> {
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
        info!("Performed LDAP user search with filter = {}", filter);
        Ok(rs.into_iter().map(SearchEntry::construct).collect())
    }

    /// Searches LDAP for groups.
    async fn search_groups(&mut self, filter: &str) -> Result<Vec<SearchEntry>, OriLDAPError> {
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
        info!("Performed LDAP group search with filter = {}", filter);
        Ok(rs.into_iter().map(SearchEntry::construct).collect())
    }

    /// Creates LDAP object with specified distinguished name and attributes.
    async fn add(
        &mut self,
        dn: &str,
        attrs: Vec<(&str, HashSet<&str>)>,
    ) -> Result<(), OriLDAPError> {
        debug!("Adding object {}", dn);
        self.ldap.add(dn, attrs).await?.success()?;
        info!("Added object {}", dn);
        Ok(())
    }

    /// Updates LDAP object with specified distinguished name and attributes.
    async fn modify(
        &mut self,
        old_dn: &str,
        new_dn: &str,
        mods: Vec<Mod<&str>>,
    ) -> Result<(), OriLDAPError> {
        debug!("Modifying object {}", old_dn);
        self.ldap.modify(old_dn, mods).await?;
        if old_dn != new_dn {
            if let Some((new_rdn, _rest)) = new_dn.split_once(',') {
                self.ldap.modifydn(old_dn, new_rdn, true, None).await?;
            }
        }
        info!("Modified object {}", old_dn);
        Ok(())
    }

    /// Deletes LDAP object with specified distinguished name.
    pub async fn delete(&mut self, dn: &str) -> Result<(), OriLDAPError> {
        debug!("Deleting object {}", dn);
        self.ldap.delete(dn).await?;
        info!("Deleted object {}", dn);
        Ok(())
    }

    // Checks if cn is available, including default LDAP admin class
    pub async fn is_username_available(&mut self, username: &str) -> bool {
        let users = self
            .search_users(&format!(
                "(&({}={})(|(objectClass={})))",
                self.config.ldap_username_attr, username, self.config.ldap_user_obj_class
            ))
            .await;
        match users {
            Ok(users) => users.is_empty(),
            _ => true,
        }
    }

    /// Retrieves user with given username from LDAP.
    /// TODO: Password must agree with the password stored in LDAP.
    pub async fn get_user(&mut self, username: &str, password: &str) -> Result<User, OriLDAPError> {
        debug!("Performing LDAP user search: {}", username);
        let mut entries = self
            .search_users(&format!(
                "(&({}={})(objectClass={}))",
                self.config.ldap_username_attr, username, self.config.ldap_user_obj_class
            ))
            .await?;
        if let Some(entry) = entries.pop() {
            info!("Performed LDAP user search: {}", username);
            Ok(User::from_searchentry(&entry, username, password))
        } else {
            Err(OriLDAPError::ObjectNotFound(format!(
                "User {} not found",
                username
            )))
        }
    }

    /// Adds user to LDAP.
    pub async fn add_user(&mut self, user: &User, password: &str) -> Result<(), OriLDAPError> {
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
    pub async fn modify_user(&mut self, username: &str, user: &User) -> Result<(), OriLDAPError> {
        debug!("Modifying user {}", username);
        let old_dn = self.config.user_dn(username);
        let new_dn = self.config.user_dn(&user.username);
        self.modify(&old_dn, &new_dn, user.as_ldap_mod(self.config))
            .await?;
        info!("Modified user {}", username);
        Ok(())
    }

    /// Deletes user from LDAP.
    pub async fn delete_user(&mut self, username: &str) -> Result<(), OriLDAPError> {
        debug!("Deleting user {}", username);
        let dn = self.config.user_dn(username);
        self.delete(&dn).await?;
        info!("Deleted user {}", username);
        Ok(())
    }

    /// Changes user password.
    pub async fn set_password(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<(), OriLDAPError> {
        debug!("Setting password for user {}", username);
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
        info!("Password set for user {}", username);
        Ok(())
    }

    /// Retrieves group with given groupname from LDAP.
    pub async fn get_group(&mut self, groupname: &str) -> Result<Group, OriLDAPError> {
        debug!("Performing LDAP group search: {}", groupname);
        let mut enties = self
            .search_groups(&format!(
                "(&({}={})(objectClass={}))",
                self.config.ldap_groupname_attr, groupname, self.config.ldap_group_obj_class
            ))
            .await?;
        if let Some(entry) = enties.pop() {
            info!("Performed LDAP user search: {}", groupname);
            Ok(Group::from_searchentry(&entry, self.config))
        } else {
            Err(OriLDAPError::ObjectNotFound(format!(
                "Group {} not found",
                groupname
            )))
        }
    }

    /// Lists users satisfying specified criteria
    pub async fn get_groups(&mut self) -> Result<Vec<Group>, OriLDAPError> {
        debug!("Performing LDAP group search");
        let mut entries = self
            .search_groups(&format!(
                "(objectClass={})",
                self.config.ldap_group_obj_class
            ))
            .await?;
        let users = entries
            .drain(..)
            .map(|entry| Group::from_searchentry(&entry, self.config))
            .collect();
        info!("Performed LDAP group search");
        Ok(users)
    }

    /// Add user to a group.
    pub async fn add_user_to_group(
        &mut self,
        username: &str,
        groupname: &str,
    ) -> Result<(), OriLDAPError> {
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
    ) -> Result<(), OriLDAPError> {
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
