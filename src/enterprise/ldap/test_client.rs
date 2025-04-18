use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use ldap3::{Mod, SearchEntry};

use super::error::LdapError;
use crate::db::User;

impl super::LDAPConnection {
    pub(crate) async fn create() -> Result<super::LDAPConnection, LdapError> {
        Ok(Self {
            config: super::LDAPConfig::default(),
            test_memberships: HashMap::new(),
            test_users: Vec::new(),
            url: String::new(),
        })
    }

    pub(super) async fn search_users(
        &mut self,
        _filter: &str,
    ) -> Result<Vec<SearchEntry>, LdapError> {
        Ok(vec![])
    }

    pub(super) async fn test_bind_user(&self, _dn: &str, _password: &str) -> Result<(), LdapError> {
        Ok(())
    }

    pub(super) async fn get_user_groups(
        &mut self,
        _user_dn: &str,
    ) -> Result<Vec<SearchEntry>, LdapError> {
        Ok(vec![])
    }

    pub(super) async fn search_groups(
        &mut self,
        _filter: &str,
    ) -> Result<Vec<SearchEntry>, LdapError> {
        Ok(vec![])
    }

    pub(super) async fn add(
        &mut self,
        _dn: &str,
        _attrs: Vec<(&str, HashSet<&str>)>,
    ) -> Result<(), LdapError> {
        Ok(())
    }

    pub(super) async fn modify<S>(
        &mut self,
        _old_dn: &str,
        _new_dn: &str,
        _mods: Vec<Mod<S>>,
    ) -> Result<(), LdapError>
    where
        S: AsRef<[u8]> + Eq + Hash,
    {
        Ok(())
    }

    pub(super) async fn delete(&mut self, _dn: &str) -> Result<(), LdapError> {
        Ok(())
    }

    pub(super) async fn get_ldap_group_memberships<'a>(
        &mut self,
        _all_ldap_users: &'a [User],
    ) -> Result<HashMap<String, HashSet<&'a User>>, LdapError> {
        Ok(HashMap::new())
    }

    pub(crate) async fn get_group_members(
        &mut self,
        _groupname: &str,
    ) -> Result<Vec<String>, LdapError> {
        Ok(vec![])
    }

    pub(super) async fn list_users(&mut self) -> Result<Vec<SearchEntry>, LdapError> {
        Ok(vec![])
    }
}
