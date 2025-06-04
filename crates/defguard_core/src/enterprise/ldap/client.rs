use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    time::Duration,
};

use ldap3::{
    adapters::PagedResults, drive, LdapConnAsync, LdapConnSettings, Mod, Scope, SearchEntry,
};

use super::error::LdapError;
use crate::{
    db::{Settings, User},
    enterprise::ldap::model::extract_rdn_value,
};

impl super::LDAPConnection {
    pub(crate) async fn create() -> Result<super::LDAPConnection, LdapError> {
        let settings = Settings::get_current_settings();
        let config = super::LDAPConfig::try_from(settings.clone())?;
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
    pub(super) async fn search_users(
        &mut self,
        filter: &str,
    ) -> Result<Vec<SearchEntry>, LdapError> {
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

    pub(super) async fn test_bind_user(&self, dn: &str, password: &str) -> Result<(), LdapError> {
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
    pub(super) async fn get_user_groups(
        &mut self,
        user_dn: &str,
    ) -> Result<Vec<SearchEntry>, LdapError> {
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
        debug!("Found groups: {rs:?}");
        Ok(rs.into_iter().map(SearchEntry::construct).collect())
    }

    /// Searches LDAP for groups.
    pub(super) async fn search_groups(
        &mut self,
        filter: &str,
    ) -> Result<Vec<SearchEntry>, LdapError> {
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
    pub(super) async fn add(
        &mut self,
        dn: &str,
        attrs: Vec<(&str, HashSet<&str>)>,
    ) -> Result<(), LdapError> {
        debug!("Adding object {dn}");
        let result = self.ldap.add(dn, attrs).await?.success()?;
        debug!("LDAP add result: {result:?}");
        info!("Added object {dn}");

        Ok(())
    }

    /// Updates LDAP object with specified distinguished name and attributes.
    pub(super) async fn modify<S>(
        &mut self,
        old_dn: &str,
        new_dn: &str,
        mods: Vec<Mod<S>>,
    ) -> Result<(), LdapError>
    where
        S: AsRef<[u8]> + Eq + Hash,
    {
        self.ldap.modify(old_dn, mods).await?;
        if old_dn != new_dn {
            if let Some((new_rdn, _rest)) = new_dn.split_once(',') {
                self.ldap.modifydn(old_dn, new_rdn, true, None).await?;
            } else {
                warn!("Failed to rename LDAP object {old_dn} to {new_dn}, new DN is invalid");
            }
        }
        info!("Modified LDAP object {old_dn}");

        Ok(())
    }

    /// Deletes LDAP object with specified distinguished name.
    pub(super) async fn delete(&mut self, dn: &str) -> Result<(), LdapError> {
        debug!("Deleting LDAP object {dn}");
        let result = self.ldap.delete(dn).await?;
        debug!("LDAP deletion result: {result:?}");
        info!("Deleted LDAP object {dn}");

        Ok(())
    }

    /// Returns a map of group names to a set of members
    pub(super) async fn get_ldap_group_memberships<'a>(
        &mut self,
        all_ldap_users: &'a [User],
    ) -> Result<HashMap<String, HashSet<&'a User>>, LdapError> {
        debug!("Retrieving LDAP group memberships");
        let mut membership_entries = self.list_group_memberships().await?;
        let mut memberships: HashMap<String, HashSet<&User>> = HashMap::new();
        // rdn: user map
        let rdn_map = all_ldap_users
            .iter()
            .map(|u| (u.ldap_rdn_value(), u))
            .collect::<HashMap<_, _>>();

        for entry in membership_entries.iter_mut() {
            let groupname = entry
                .attrs
                .remove(&self.config.ldap_groupname_attr)
                .and_then(|mut v| v.pop());

            if let Some(groupname) = groupname {
                if let Some(members) = entry.attrs.get(&self.config.ldap_group_member_attr) {
                    let members = members
                        .iter()
                        .filter_map(|v| {
                            extract_rdn_value(v).and_then(|v| {
                                if let Some(user) = rdn_map.get(v.as_str()) {
                                    Some(*user)
                                } else {
                                    debug!(
                                        "LDAP group {groupname} contains member {v} that does not belong to the filtered LDAP users list, skipping"
                                    );
                                    None
                                }

                            })
                        })
                        .collect::<HashSet<_>>();
                    memberships.insert(groupname, members);
                } else {
                    warn!("LDAP group {groupname} missing group member attribute, skipping");
                }
            } else {
                warn!("Group entry {entry:?} missing groupname attribute, skipping");
            }
        }

        Ok(memberships)
    }

    pub(super) async fn is_member_of(
        &mut self,
        user_dn: &str,
        groupname: &str,
    ) -> Result<bool, LdapError> {
        debug!("Checking if user {user_dn} is member of group {groupname}");
        let filter = format!(
            "(&(objectClass={})({}={})({}={}))",
            self.config.ldap_group_obj_class,
            self.config.ldap_groupname_attr,
            groupname,
            self.config.ldap_group_member_attr,
            user_dn
        );
        debug!(
            "Using the following filter for group search: {filter} and base: {}",
            self.config.ldap_group_search_base
        );
        let (rs, res) = self
            .ldap
            .search(
                &self.config.ldap_group_search_base,
                Scope::Subtree,
                filter.as_str(),
                vec!["*"],
            )
            .await?
            .success()?;
        debug!("LDAP group membership search result: {res:?}");
        Ok(!rs.is_empty())
    }

    pub(super) async fn get_group_members(
        &mut self,
        groupname: &str,
    ) -> Result<Vec<String>, LdapError> {
        debug!("Searching for group memberships for group {}", groupname);
        let filter = format!(
            "(&(objectClass={})({}={}))",
            self.config.ldap_group_obj_class, self.config.ldap_groupname_attr, groupname
        );
        debug!(
            "Using the following filter for group search: {filter} and base: {}",
            self.config.ldap_group_search_base
        );
        let mut search_stream = self
            .ldap
            .streaming_search_with(
                PagedResults::new(500),
                &self.config.ldap_group_search_base,
                Scope::Subtree,
                filter.as_str(),
                vec![&self.config.ldap_group_member_attr],
            )
            .await?;

        let mut member_entries = Vec::new();
        while let Some(entry) = search_stream.next().await? {
            member_entries.push(SearchEntry::construct(entry));
        }

        let members = member_entries
            .first()
            .and_then(|entry| {
                let member_entries = entry.attrs.get(&self.config.ldap_group_member_attr);
                member_entries.map(|v| {
                    v.iter()
                        .filter_map(|v| extract_rdn_value(v))
                        .collect::<Vec<_>>()
                })
            })
            .unwrap_or_default();
        debug!(
            "Performed LDAP group memberships search for group {}",
            groupname
        );

        Ok(members)
    }

    pub(super) async fn list_users(&mut self) -> Result<Vec<SearchEntry>, LdapError> {
        let filter = if !self.config.ldap_sync_groups.is_empty() {
            debug!(
                "LDAP sync groups are defined, filtering users by those groups: {:?}",
                self.config.ldap_sync_groups
            );
            let mut group_filters = vec![];
            for group in self.config.ldap_sync_groups.iter() {
                let group_dn = self.config.group_dn(group);
                group_filters.push(format!("({}={})", self.config.ldap_member_attr, group_dn));
            }
            debug!(
                "Using the following group filters for user search: {:?}",
                group_filters
            );
            format!(
                "(&(objectClass={})(|{}))",
                self.config.ldap_user_obj_class,
                group_filters.join("")
            )
        } else {
            debug!("No LDAP sync groups defined, searching for all users in the base DN");
            format!("(objectClass={})", self.config.ldap_user_obj_class)
        };

        debug!(
            "Using the following filter for user search: {filter} and base: {}",
            self.config.ldap_user_search_base
        );

        let mut search_stream = self
            .ldap
            .streaming_search_with(
                PagedResults::new(500),
                &self.config.ldap_user_search_base,
                Scope::Subtree,
                &filter,
                vec!["*", &self.config.ldap_member_attr],
            )
            .await?;

        let mut entries = vec![];
        while let Some(entry) = search_stream.next().await? {
            entries.push(SearchEntry::construct(entry));
        }

        debug!("Performed LDAP user search");

        Ok(entries)
    }

    pub(super) async fn list_group_memberships(&mut self) -> Result<Vec<SearchEntry>, LdapError> {
        debug!("Searching for group memberships");
        let filter = format!(
            "(&(objectClass={})({}=*))",
            self.config.ldap_group_obj_class, self.config.ldap_group_member_attr
        );
        debug!(
            "Using the following filter for group search: {filter} and base: {}",
            self.config.ldap_group_search_base
        );
        let mut search_stream = self
            .ldap
            .streaming_search_with(
                PagedResults::new(500),
                &self.config.ldap_group_search_base,
                Scope::Subtree,
                filter.as_str(),
                vec![
                    &self.config.ldap_groupname_attr,
                    &self.config.ldap_group_member_attr,
                ],
            )
            .await?;

        let mut memberships = Vec::new();
        while let Some(entry) = search_stream.next().await? {
            memberships.push(SearchEntry::construct(entry));
        }

        debug!("Performed LDAP group memberships search");

        Ok(memberships)
    }
}
