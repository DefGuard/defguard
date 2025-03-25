use std::collections::{HashMap, HashSet};

use ldap3::{Scope, SearchEntry};
use sqlx::{PgConnection, PgPool, Type};

use crate::{
    db::{models::settings::update_current_settings, Group, Id, Settings, User},
    ldap::{error::LdapError, model::extract_dn_value},
};

async fn get_or_create_group(
    transaction: &mut PgConnection,
    groupname: &str,
) -> Result<Group<Id>, LdapError> {
    let group = if let Some(group) = Group::find_by_name(&mut *transaction, groupname).await? {
        debug!("Group {groupname} already exists, skipping creation");
        group
    } else {
        debug!("Group {groupname} didn't exist, creating it now");
        let new_group = Group::new(groupname).save(&mut *transaction).await?;
        debug!("Group {groupname} created");
        new_group
    };

    Ok(group)
}

#[derive(Debug, Clone, Copy)]
pub enum Source {
    LDAP,
    Defguard,
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Deserialize, Serialize, Default, Type)]
#[sqlx(type_name = "ldap_sync_status", rename_all = "lowercase")]
pub enum SyncStatus {
    Synced,
    #[default]
    Desynced,
}

impl SyncStatus {
    pub fn is_out_of_sync(&self) -> bool {
        matches!(self, SyncStatus::Desynced)
    }
}

pub fn get_ldap_sync_status() -> SyncStatus {
    let settings = Settings::get_current_settings();
    settings.ldap_sync_status
}

pub async fn set_ldap_sync_status(status: SyncStatus, pool: &PgPool) -> Result<(), LdapError> {
    debug!("Setting LDAP sync status to {status:?}");
    let mut settings = Settings::get_current_settings();
    settings.ldap_sync_status = status;
    update_current_settings(pool, settings).await?;
    debug!("LDAP sync status set to {status:?}");
    Ok(())
}

pub fn is_ldap_desynced() -> bool {
    get_ldap_sync_status().is_out_of_sync()
}

#[derive(Debug)]
struct UserSyncChanges {
    pub delete_defguard: Vec<User<Id>>,
    pub add_defguard: Vec<User>,
    pub delete_ldap: Vec<User>,
    pub add_ldap: Vec<User<Id>>,
}

fn compute_user_sync_changes(
    all_ldap_users: Vec<User>,
    all_defguard_users: Vec<User<Id>>,
    authority: Source,
) -> UserSyncChanges {
    debug!("Computing user sync changes (user creation/deletion), authority: {authority:?}");
    let mut delete_defguard = Vec::new();
    let mut add_defguard = Vec::new();
    let mut delete_ldap = Vec::new();
    let mut add_ldap = Vec::new();

    let mut ldap_usernames = HashSet::with_capacity(all_ldap_users.len());
    let defguard_usernames: HashSet<&str> = all_defguard_users
        .iter()
        .map(|u| u.username.as_str())
        .collect();

    trace!("Defguard users: {:?}", defguard_usernames);
    trace!("LDAP users: {:?}", all_ldap_users);

    for user in all_ldap_users {
        ldap_usernames.insert(user.username.clone());

        debug!("Checking if user {} is in Defguard", user.username);
        if !defguard_usernames.contains(user.username.as_str()) {
            debug!("User {} not found in Defguard", user.username);
            match authority {
                Source::LDAP => add_defguard.push(user),
                Source::Defguard => delete_ldap.push(user),
            }
        }
    }

    for user in all_defguard_users {
        debug!("Checking if user {} is in LDAP", user.username);
        if !ldap_usernames.contains(&user.username) {
            debug!("User {} not found in LDAP", user.username);
            match authority {
                Source::LDAP => {
                    // Skip inactive/not enrolled users when deleting from LDAP
                    if user.is_active && user.is_enrolled() {
                        debug!(
                            "User {} is active and enrolled, removing from Defguard",
                            user.username
                        );
                        delete_defguard.push(user);
                    } else {
                        debug!(
                            "User {} is inactive or not enrolled, skipping deletion from Defguard",
                            user.username
                        );
                    }
                }
                Source::Defguard => {
                    // Skip inactive users when adding to LDAP
                    if user.is_active && user.is_enrolled() {
                        debug!(
                            "User {} is active and enrolled, adding to LDAP",
                            user.username
                        );
                        add_ldap.push(user);
                    } else {
                        debug!(
                            "User {} is inactive or not enrolled, skipping addition to LDAP",
                            user.username
                        );
                    }
                }
            }
        }
    }

    let user_sync_changes = UserSyncChanges {
        delete_defguard,
        add_defguard,
        delete_ldap,
        add_ldap,
    };

    debug!("Completed computing user sync changes");
    trace!("User sync changes: {:?}", user_sync_changes);

    user_sync_changes
}

#[derive(Debug)]
struct GroupSyncChanges {
    pub add_defguard: HashMap<String, HashSet<String>>,
    pub delete_defguard: HashMap<String, HashSet<String>>,
    pub add_ldap: HashMap<String, HashSet<String>>,
    pub delete_ldap: HashMap<String, HashSet<String>>,
}

fn compute_group_sync_changes(
    defguard_memberships: HashMap<String, HashSet<String>>,
    ldap_memberships: HashMap<String, HashSet<String>>,
    authority: Source,
) -> GroupSyncChanges {
    debug!("Computing group sync changes (group membership changes), authority: {authority:?}");
    let mut delete_defguard = HashMap::new();
    let mut add_defguard = HashMap::new();
    let mut delete_ldap = HashMap::new();
    let mut add_ldap = HashMap::new();

    for (group, members) in defguard_memberships.clone() {
        debug!("Checking group {} for changes", group);
        if !ldap_memberships.contains_key(&group) {
            debug!("Group {group:?} is missing from LDAP");
            match authority {
                Source::Defguard => add_ldap.insert(group.clone(), members.clone()),
                Source::LDAP => delete_defguard.insert(group.clone(), members.clone()),
            };
        } else {
            debug!("Group {group:?} found in LDAP, checking for membership differences");
            let ldap_members = ldap_memberships.get(&group).unwrap();
            let missing_from_defguard = ldap_members
                .difference(&members)
                .cloned()
                .collect::<HashSet<_>>();

            let missing_from_ldap = members
                .difference(ldap_members)
                .cloned()
                .collect::<HashSet<_>>();

            trace!(
                "Group {group:?} members missing from Defguard: {missing_from_defguard:?}, missing from LDAP: {missing_from_ldap:?}"
            );

            if !missing_from_defguard.is_empty() {
                match authority {
                    Source::Defguard => delete_ldap.insert(group.clone(), missing_from_defguard),
                    Source::LDAP => add_defguard.insert(group.clone(), missing_from_defguard),
                };
            }

            if !missing_from_ldap.is_empty() {
                match authority {
                    Source::Defguard => add_ldap.insert(group.clone(), missing_from_ldap),
                    Source::LDAP => delete_defguard.insert(group.clone(), missing_from_ldap),
                };
            }
        }
    }

    for (group, members) in ldap_memberships {
        if !defguard_memberships.contains_key(&group) {
            debug!("Group {group:?} is missing from Defguard");
            match authority {
                Source::Defguard => delete_ldap.insert(group.clone(), members.clone()),
                Source::LDAP => add_defguard.insert(group.clone(), members.clone()),
            };
        }
    }

    let sync_changes = GroupSyncChanges {
        add_defguard,
        delete_defguard,
        add_ldap,
        delete_ldap,
    };

    debug!("Completed computing group sync changes");
    trace!("Group sync changes: {:?}", sync_changes);

    sync_changes
}

fn attrs_different(defguard_user: &User<Id>, ldap_user: &User) -> bool {
    defguard_user.last_name != ldap_user.last_name
        || defguard_user.first_name != ldap_user.first_name
        || defguard_user.email != ldap_user.email
        || defguard_user.phone != ldap_user.phone
}

fn extract_intersecting_users(
    defguard_users: &mut Vec<User<Id>>,
    ldap_users: &mut Vec<User>,
) -> Vec<(User, User<Id>)> {
    let mut intersecting_users = vec![];
    let mut intersecting_users_ldap = vec![];

    for defguard_user in defguard_users.iter_mut() {
        if let Some(ldap_user) = ldap_users
            .iter()
            .position(|u| u.username == defguard_user.username)
            .map(|i| ldap_users.remove(i))
        {
            intersecting_users_ldap.push(ldap_user);
        }
    }

    for user in intersecting_users_ldap.into_iter() {
        if let Some(defguard_user) = defguard_users
            .iter()
            .position(|u| u.username == user.username)
            .map(|i| defguard_users.remove(i))
        {
            intersecting_users.push((user, defguard_user));
        }
    }

    intersecting_users
}

impl crate::ldap::LDAPConnection {
    async fn apply_user_modifications(
        &mut self,
        mut intersecting_users: Vec<(User, User<Id>)>,
        authority: Source,
        pool: &PgPool,
    ) -> Result<(), LdapError> {
        let mut transaction = pool.begin().await?;

        for (ldap_user, defguard_user) in intersecting_users.iter_mut() {
            if attrs_different(defguard_user, ldap_user) {
                debug!(
                    "User {} attributes differ between LDAP and Defguard, merging...",
                    defguard_user.username
                );
                match authority {
                    Source::LDAP => {
                        debug!("Applying LDAP user attributes to Defguard user");
                        defguard_user.update_from_ldap_user(ldap_user);
                        defguard_user.save(&mut *transaction).await?;
                    }
                    Source::Defguard => {
                        debug!("Applying Defguard user attributes to LDAP user");
                        self.modify_user(&defguard_user.username, defguard_user)
                            .await?;
                    }
                }
            }
        }

        transaction.commit().await?;

        Ok(())
    }

    pub(crate) async fn sync(&mut self, pool: &PgPool, full: bool) -> Result<(), LdapError> {
        let settings = Settings::get_current_settings();
        let authority = if full {
            let settings_authority = if settings.ldap_is_authoritative {
                Source::LDAP
            } else {
                Source::Defguard
            };
            debug!(
                "Full LDAP sync requested, using the following authority: {settings_authority:?}"
            );
            settings_authority
        } else {
            debug!("Incremental LDAP sync requested.");
            Source::LDAP
        };

        let all_entries = self.list_users().await?;
        let mut all_ldap_users = vec![];
        let mut all_defguard_users = User::all(pool).await?;

        for entry in all_entries {
            let username = entry
                .attrs
                .get("cn")
                .and_then(|v| v.first())
                .ok_or_else(|| LdapError::ObjectNotFound("No cn attribute found".to_string()))?;

            match User::from_searchentry(&entry, username, None) {
                Ok(user) => all_ldap_users.push(user),
                Err(err) => warn!(
                    "Failed to create user {} from LDAP entry: {:?}, error: {}. The user will be skipped during sync",
                    username, entry, err
                ),
            }
        }

        let intersecting_users =
            extract_intersecting_users(&mut all_defguard_users, &mut all_ldap_users);
        self.apply_user_modifications(intersecting_users, authority, pool)
            .await?;

        let user_changes = compute_user_sync_changes(all_ldap_users, all_defguard_users, authority);

        let ldap_memberships = self.get_ldap_group_memberships().await?;
        let mut defguard_memberships = HashMap::new();

        let defguard_groups = Group::all(pool).await?;
        for group in defguard_groups {
            let members = group
                .members(pool)
                .await?
                .into_iter()
                .filter_map(|u| if u.is_active { Some(u.username) } else { None })
                .collect::<HashSet<_>>();
            defguard_memberships.insert(group.name, members);
        }

        let membership_changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, authority);

        self.apply_user_sync_changes(pool, user_changes).await?;
        self.apply_user_group_sync_changes(pool, membership_changes)
            .await?;

        if full {
            debug!("Full LDAP sync completed");
        } else {
            debug!("LDAP Incremental sync completed");
        }

        Ok(())
    }

    async fn apply_user_group_sync_changes(
        &mut self,
        pool: &PgPool,
        changes: GroupSyncChanges,
    ) -> Result<(), LdapError> {
        debug!("Applying group memberships sync changes");
        let mut transaction = pool.begin().await?;
        let mut admin_count = User::find_admins(&mut *transaction).await?.len();
        for (groupname, members) in changes.delete_defguard {
            let group = get_or_create_group(&mut transaction, &groupname).await?;

            for member in members {
                let user = User::find_by_username(&mut *transaction, &member)
                    .await?
                    .ok_or_else(|| {
                        LdapError::ObjectNotFound(format!("User {} not found", member))
                    })?;

                if user.is_admin(&mut *transaction).await? {
                    if admin_count == 1 {
                        debug!(
                            "Cannot remove last admin user {} from Defguard. User won't be removed from group {}.",
                            user.username, groupname
                        );
                        continue;
                    } else {
                        debug!(
                            "Removing admin user {} from group {}",
                            user.username, groupname
                        );
                        admin_count -= 1;
                        user.remove_from_group(&mut *transaction, &group).await?;
                    }
                } else {
                    debug!("Removing user {} from group {}", user.username, groupname);
                    user.remove_from_group(&mut *transaction, &group).await?;
                }
            }
        }

        for (groupname, members) in changes.add_defguard {
            let group = get_or_create_group(&mut transaction, &groupname).await?;
            for member in members {
                if let Some(user) = User::find_by_username(&mut *transaction, &member).await? {
                    user.add_to_group(&mut *transaction, &group).await?;
                } else {
                    warn!(
                        "LDAP user {} not found in Defguard, despite completing user sync earlier. \
                        Your LDAP may have dangling group members. Skipping adding user to group {}",
                        member, groupname
                    );
                }
            }
        }

        transaction.commit().await?;

        for (groupname, members) in changes.delete_ldap {
            for member in members {
                self.remove_user_from_group(&member, &groupname).await?;
            }
        }

        for (groupname, members) in changes.add_ldap {
            for member in members {
                self.add_user_to_group(&member, &groupname).await?;
            }
        }

        Ok(())
    }

    async fn apply_user_sync_changes(
        &mut self,
        pool: &PgPool,
        changes: UserSyncChanges,
    ) -> Result<(), LdapError> {
        let mut transaction = pool.begin().await?;
        let mut admin_count = User::find_admins(&mut *transaction).await?.len();
        for user in changes.delete_defguard {
            if user.is_admin(&mut *transaction).await? {
                if admin_count == 1 {
                    debug!(
                        "Cannot delete last admin user from Defguard. User {} won't be deleted.",
                        user.username
                    );
                    continue;
                } else {
                    admin_count -= 1;
                    debug!("Deleting admin user {} from Defguard", user.username);
                    user.delete(&mut *transaction).await?;
                }
            } else {
                debug!("Deleting user {} from Defguard", user.username);
                user.delete(&mut *transaction).await?;
            }
        }

        for user in changes.add_defguard {
            debug!("Adding user {} to Defguard", user.username);
            user.save(&mut *transaction).await?;
        }

        transaction.commit().await?;

        for user in changes.delete_ldap {
            debug!("Deleting user {} from LDAP", user.username);
            self.delete_user(&user.username).await?;
        }

        for user in changes.add_ldap {
            debug!("Adding user {} to LDAP", user.username);
            self.add_user(&user, None).await?;
        }

        Ok(())
    }

    async fn list_users(&mut self) -> Result<Vec<SearchEntry>, LdapError> {
        let (rs, _res) = self
            .ldap
            .search(
                &self.config.ldap_user_search_base,
                Scope::Subtree,
                format!("(objectClass={})", self.config.ldap_user_obj_class).as_str(),
                vec!["*", &self.config.ldap_member_attr],
            )
            .await?
            .success()?;
        debug!("Performed LDAP user search");

        Ok(rs.into_iter().map(SearchEntry::construct).collect())
    }

    /// Returns a map of group names to a set of member usernames
    async fn get_ldap_group_memberships(
        &mut self,
    ) -> Result<HashMap<String, HashSet<String>>, LdapError> {
        let mut membership_entries = self.list_group_memberships().await?;

        let mut memberships: HashMap<String, HashSet<String>> = HashMap::new();

        for entry in membership_entries.iter_mut() {
            let groupname = entry
                .attrs
                .remove(&self.config.ldap_groupname_attr)
                .and_then(|mut v| v.pop())
                .ok_or_else(|| {
                    LdapError::ObjectNotFound(format!(
                        "No {} attribute found",
                        self.config.ldap_groupname_attr
                    ))
                })?;

            let members = entry
                .attrs
                .get(&self.config.ldap_group_member_attr)
                .ok_or_else(|| {
                    LdapError::ObjectNotFound(format!(
                        "No {} attribute found",
                        self.config.ldap_group_member_attr
                    ))
                })?
                .iter()
                .filter_map(|member| extract_dn_value(member))
                .collect::<HashSet<String>>();
            memberships.insert(groupname, members);
        }

        Ok(memberships)
    }

    async fn list_group_memberships(&mut self) -> Result<Vec<SearchEntry>, LdapError> {
        let (rs, _res) = self
            .ldap
            .search(
                &self.config.ldap_group_search_base,
                Scope::Subtree,
                "(objectClass=groupOfUniqueNames)",
                vec!["cn", "uniqueMember"],
            )
            .await?
            .success()?;

        let memberships = rs.into_iter().map(SearchEntry::construct).collect();
        debug!("Performed LDAP group memberships search");

        Ok(memberships)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_user_sync_changes_empty_lists() {
        let ldap_users: Vec<User> = vec![];
        let defguard_users: Vec<User<Id>> = vec![];

        let changes = compute_user_sync_changes(ldap_users, defguard_users, Source::LDAP);

        assert!(changes.delete_defguard.is_empty());
        assert!(changes.add_defguard.is_empty());
        assert!(changes.delete_ldap.is_empty());
        assert!(changes.add_ldap.is_empty());
    }

    #[test]
    fn test_ldap_authority_add_to_defguard() {
        let ldap_user = User::new(
            "test_user",
            Some("test_password"),
            "last name",
            "first name",
            "email@email.com",
            None,
        );

        let ldap_users = vec![ldap_user];
        let defguard_users: Vec<User<Id>> = vec![];

        let changes = compute_user_sync_changes(ldap_users, defguard_users, Source::LDAP);

        assert!(changes.delete_defguard.is_empty());
        assert_eq!(changes.add_defguard.len(), 1);
        assert_eq!(changes.add_defguard[0].username, "test_user");
        assert!(changes.delete_ldap.is_empty());
        assert!(changes.add_ldap.is_empty());
    }

    #[sqlx::test]
    fn test_ldap_authority_delete_from_defguard(pool: PgPool) {
        let defguard_user = User::new(
            "test_user",
            Some("test_password"),
            "last name",
            "first name",
            "email@email.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let ldap_users: Vec<User> = vec![];
        let defguard_users = vec![defguard_user];

        let changes = compute_user_sync_changes(ldap_users, defguard_users, Source::LDAP);

        assert_eq!(changes.delete_defguard.len(), 1);
        assert_eq!(changes.delete_defguard[0].username, "test_user");
        assert!(changes.add_defguard.is_empty());
        assert!(changes.delete_ldap.is_empty());
        assert!(changes.add_ldap.is_empty());
    }

    #[sqlx::test]
    fn test_defguard_authority_add_to_ldap(pool: PgPool) {
        let defguard_user = User::new(
            "test_user",
            Some("test_password"),
            "last name",
            "first name",
            "email@email.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let ldap_users: Vec<User> = vec![];
        let defguard_users = vec![defguard_user];

        let changes = compute_user_sync_changes(ldap_users, defguard_users, Source::Defguard);

        assert!(changes.delete_defguard.is_empty());
        assert!(changes.add_defguard.is_empty());
        assert!(changes.delete_ldap.is_empty());
        assert_eq!(changes.add_ldap.len(), 1);
        assert_eq!(changes.add_ldap[0].username, "test_user");
    }

    #[test]
    fn test_defguard_authority_delete_from_ldap() {
        let ldap_user = User::new(
            "test_user",
            Some("test_password"),
            "last name",
            "first name",
            "email@email.com",
            None,
        );

        let ldap_users = vec![ldap_user];
        let defguard_users: Vec<User<Id>> = vec![];

        let changes = compute_user_sync_changes(ldap_users, defguard_users, Source::Defguard);

        assert!(changes.delete_defguard.is_empty());
        assert!(changes.add_defguard.is_empty());
        assert_eq!(changes.delete_ldap.len(), 1);
        assert_eq!(changes.delete_ldap[0].username, "test_user");
        assert!(changes.add_ldap.is_empty());
    }

    #[sqlx::test]
    fn test_matching_users_no_changes(pool: PgPool) {
        let ldap_user = User::new(
            "test_user",
            Some("test_password"),
            "last name",
            "first name",
            "email@email.com",
            None,
        );

        let defguard_user = User::new(
            "test_user",
            Some("test_password"),
            "last name",
            "first name",
            "email@email.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let ldap_users = vec![ldap_user];
        let defguard_users = vec![defguard_user];

        let changes_ldap =
            compute_user_sync_changes(ldap_users.clone(), defguard_users.clone(), Source::LDAP);

        assert!(changes_ldap.delete_defguard.is_empty());
        assert!(changes_ldap.add_defguard.is_empty());
        assert!(changes_ldap.delete_ldap.is_empty());
        assert!(changes_ldap.add_ldap.is_empty());

        let changes_defguard =
            compute_user_sync_changes(ldap_users, defguard_users, Source::Defguard);

        assert!(changes_defguard.delete_defguard.is_empty());
        assert!(changes_defguard.add_defguard.is_empty());
        assert!(changes_defguard.delete_ldap.is_empty());
        assert!(changes_defguard.add_ldap.is_empty());
    }

    #[test]
    fn test_compute_group_sync_changes_empty_maps() {
        let defguard_memberships = HashMap::new();
        let ldap_memberships = HashMap::new();

        let changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Source::LDAP);

        assert!(changes.delete_defguard.is_empty());
        assert!(changes.add_defguard.is_empty());
        assert!(changes.delete_ldap.is_empty());
        assert!(changes.add_ldap.is_empty());
    }

    #[test]
    fn test_ldap_authority_add_group_to_defguard() {
        let defguard_memberships = HashMap::new();
        let mut ldap_memberships = HashMap::new();
        ldap_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec!["user1".to_string()]),
        );

        let changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Source::LDAP);

        assert!(changes.delete_defguard.is_empty());
        assert_eq!(changes.add_defguard.len(), 1);
        assert!(changes.add_defguard.contains_key("test_group"));
        assert_eq!(changes.add_defguard["test_group"].len(), 1);
        assert!(changes.add_defguard["test_group"].contains("user1"));
        assert!(changes.delete_ldap.is_empty());
        assert!(changes.add_ldap.is_empty());
    }

    #[test]
    fn test_ldap_authority_delete_group_from_defguard() {
        let mut defguard_memberships = HashMap::new();
        defguard_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec!["user1".to_string()]),
        );
        let ldap_memberships = HashMap::new();

        let changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Source::LDAP);

        assert_eq!(changes.delete_defguard.len(), 1);
        assert!(changes.delete_defguard.contains_key("test_group"));
        assert_eq!(changes.delete_defguard["test_group"].len(), 1);
        assert!(changes.delete_defguard["test_group"].contains("user1"));
        assert!(changes.add_defguard.is_empty());
        assert!(changes.delete_ldap.is_empty());
        assert!(changes.add_ldap.is_empty());
    }

    #[test]
    fn test_defguard_authority_add_group_to_ldap() {
        let mut defguard_memberships = HashMap::new();
        defguard_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec!["user1".to_string()]),
        );
        let ldap_memberships = HashMap::new();

        let changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Source::Defguard);

        assert!(changes.delete_defguard.is_empty());
        assert!(changes.add_defguard.is_empty());
        assert!(changes.delete_ldap.is_empty());
        assert_eq!(changes.add_ldap.len(), 1);
        assert!(changes.add_ldap.contains_key("test_group"));
        assert_eq!(changes.add_ldap["test_group"].len(), 1);
        assert!(changes.add_ldap["test_group"].contains("user1"));
    }

    #[test]
    fn test_defguard_authority_delete_group_from_ldap() {
        let defguard_memberships = HashMap::new();
        let mut ldap_memberships = HashMap::new();
        ldap_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec!["user1".to_string()]),
        );

        let changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Source::Defguard);

        assert!(changes.delete_defguard.is_empty());
        assert!(changes.add_defguard.is_empty());
        assert_eq!(changes.delete_ldap.len(), 1);
        assert!(changes.delete_ldap.contains_key("test_group"));
        assert_eq!(changes.delete_ldap["test_group"].len(), 1);
        assert!(changes.delete_ldap["test_group"].contains("user1"));
        assert!(changes.add_ldap.is_empty());
    }

    #[test]
    fn test_matching_groups_no_changes() {
        let mut defguard_memberships = HashMap::new();
        defguard_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec!["user1".to_string()]),
        );
        let mut ldap_memberships = HashMap::new();
        ldap_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec!["user1".to_string()]),
        );

        let changes_ldap = compute_group_sync_changes(
            defguard_memberships.clone(),
            ldap_memberships.clone(),
            Source::LDAP,
        );

        // Since members are identical, these should be empty
        assert!(
            changes_ldap.delete_defguard.is_empty()
                || changes_ldap.delete_defguard["test_group"].is_empty()
        );
        assert!(
            changes_ldap.add_defguard.is_empty()
                || changes_ldap.add_defguard["test_group"].is_empty()
        );
        assert!(
            changes_ldap.delete_ldap.is_empty()
                || changes_ldap.delete_ldap["test_group"].is_empty()
        );
        assert!(changes_ldap.add_ldap.is_empty() || changes_ldap.add_ldap["test_group"].is_empty());

        let changes_defguard =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Source::Defguard);

        // Since members are identical, these should be empty
        assert!(
            changes_defguard.delete_defguard.is_empty()
                || changes_defguard.delete_defguard["test_group"].is_empty()
        );
        assert!(
            changes_defguard.add_defguard.is_empty()
                || changes_defguard.add_defguard["test_group"].is_empty()
        );
        assert!(
            changes_defguard.delete_ldap.is_empty()
                || changes_defguard.delete_ldap["test_group"].is_empty()
        );
        assert!(
            changes_defguard.add_ldap.is_empty()
                || changes_defguard.add_ldap["test_group"].is_empty()
        );
    }

    #[test]
    fn test_ldap_authority_add_users_to_group() {
        let mut defguard_memberships = HashMap::new();
        defguard_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec!["user1".to_string()]),
        );
        let mut ldap_memberships = HashMap::new();
        ldap_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec!["user1".to_string(), "user2".to_string()]),
        );

        let changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Source::LDAP);

        assert!(changes.add_defguard.contains_key("test_group"));
        assert_eq!(changes.add_defguard["test_group"].len(), 1);
        assert!(changes.add_defguard["test_group"].contains("user2"));
    }

    #[test]
    fn test_ldap_authority_remove_users_from_group() {
        let mut defguard_memberships = HashMap::new();
        defguard_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec!["user1".to_string(), "user2".to_string()]),
        );
        let mut ldap_memberships = HashMap::new();
        ldap_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec!["user1".to_string()]),
        );

        let changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Source::LDAP);

        assert!(changes.delete_defguard.contains_key("test_group"));
        assert_eq!(changes.delete_defguard["test_group"].len(), 1);
        assert!(changes.delete_defguard["test_group"].contains("user2"));
    }

    #[test]
    fn test_multiple_groups_ldap_authority() {
        let mut defguard_memberships = HashMap::new();
        defguard_memberships.insert(
            "group1".to_string(),
            HashSet::from_iter(vec!["user1".to_string(), "user2".to_string()]),
        );
        defguard_memberships.insert(
            "group2".to_string(),
            HashSet::from_iter(vec!["user3".to_string()]),
        );

        let mut ldap_memberships = HashMap::new();
        ldap_memberships.insert(
            "group1".to_string(),
            HashSet::from_iter(vec!["user1".to_string(), "user4".to_string()]),
        );
        ldap_memberships.insert(
            "group3".to_string(),
            HashSet::from_iter(vec!["user5".to_string(), "user6".to_string()]),
        );

        let changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Source::LDAP);

        // group1: remove user2, add user4
        assert!(changes.delete_defguard.contains_key("group1"));
        assert_eq!(changes.delete_defguard["group1"].len(), 1);
        assert!(changes.delete_defguard["group1"].contains("user2"));
        assert!(changes.add_defguard.contains_key("group1"));
        assert_eq!(changes.add_defguard["group1"].len(), 1);
        assert!(changes.add_defguard["group1"].contains("user4"));

        // group2: should be deleted entirely
        assert!(changes.delete_defguard.contains_key("group2"));
        assert_eq!(changes.delete_defguard["group2"].len(), 1);
        assert!(changes.delete_defguard["group2"].contains("user3"));

        // group3: should be added entirely
        assert!(changes.add_defguard.contains_key("group3"));
        assert_eq!(changes.add_defguard["group3"].len(), 2);
        assert!(changes.add_defguard["group3"].contains("user5"));
        assert!(changes.add_defguard["group3"].contains("user6"));

        // Nothing should be changed in LDAP since we use LDAP as authority
        assert!(changes.delete_ldap.is_empty());
        assert!(changes.add_ldap.is_empty());
    }

    #[test]
    fn test_multiple_groups_defguard_authority() {
        let mut defguard_memberships = HashMap::new();
        defguard_memberships.insert(
            "group1".to_string(),
            HashSet::from_iter(vec!["user1".to_string(), "user2".to_string()]),
        );
        defguard_memberships.insert(
            "group3".to_string(),
            HashSet::from_iter(vec!["user5".to_string(), "user6".to_string()]),
        );

        let mut ldap_memberships = HashMap::new();
        ldap_memberships.insert(
            "group1".to_string(),
            HashSet::from_iter(vec!["user1".to_string(), "user4".to_string()]),
        );
        ldap_memberships.insert(
            "group2".to_string(),
            HashSet::from_iter(vec!["user3".to_string()]),
        );

        let changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Source::Defguard);

        assert!(changes.delete_defguard.is_empty());
        assert!(changes.add_defguard.is_empty());

        // group1: add user2, remove user4
        assert!(changes.add_ldap.contains_key("group1"));
        assert_eq!(changes.add_ldap["group1"].len(), 1);
        assert!(changes.add_ldap["group1"].contains("user2"));
        assert!(changes.delete_ldap.contains_key("group1"));
        assert_eq!(changes.delete_ldap["group1"].len(), 1);
        assert!(changes.delete_ldap["group1"].contains("user4"));

        // group2: should be deleted entirely
        assert!(changes.delete_ldap.contains_key("group2"));
        assert_eq!(changes.delete_ldap["group2"].len(), 1);
        assert!(changes.delete_ldap["group2"].contains("user3"));

        // group3: should be added entirely to LDAP
        assert!(changes.add_ldap.contains_key("group3"));
        assert_eq!(changes.add_ldap["group3"].len(), 2);
        assert!(changes.add_ldap["group3"].contains("user5"));
        assert!(changes.add_ldap["group3"].contains("user6"));
    }

    #[test]
    fn test_empty_groups() {
        let mut defguard_memberships = HashMap::new();
        defguard_memberships.insert("empty_group1".to_string(), HashSet::new());

        let mut ldap_memberships = HashMap::new();
        ldap_memberships.insert("empty_group2".to_string(), HashSet::new());

        let changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Source::LDAP);

        // empty_group1 should be deleted from defguard (it's not in LDAP)
        assert!(changes.delete_defguard.contains_key("empty_group1"));
        assert_eq!(changes.delete_defguard["empty_group1"].len(), 0);
        assert!(changes.delete_defguard["empty_group1"].is_empty());

        // empty_group2 should be added to defguard (it's in LDAP)
        assert!(changes.add_defguard.contains_key("empty_group2"));
        assert_eq!(changes.add_defguard["empty_group2"].len(), 0);
        assert!(changes.add_defguard["empty_group2"].is_empty());
    }

    #[test]
    fn test_complex_group_memberships() {
        let mut defguard_memberships = HashMap::new();
        defguard_memberships.insert(
            "group1".to_string(),
            HashSet::from_iter(vec!["user1".to_string(), "user2".to_string()]),
        );
        defguard_memberships.insert(
            "group2".to_string(),
            HashSet::from_iter(vec![
                "user1".to_string(),
                "user2".to_string(),
                "user3".to_string(),
            ]),
        );
        defguard_memberships.insert(
            "group3".to_string(),
            HashSet::from_iter(vec!["user1".to_string(), "user5".to_string()]),
        );

        let mut ldap_memberships = HashMap::new();
        ldap_memberships.insert(
            "group1".to_string(),
            HashSet::from_iter(vec!["user1".to_string(), "user4".to_string()]),
        );
        ldap_memberships.insert(
            "group2".to_string(),
            HashSet::from_iter(vec![
                "user1".to_string(),
                "user2".to_string(),
                "user4".to_string(),
            ]),
        );
        ldap_memberships.insert(
            "group4".to_string(),
            HashSet::from_iter(vec!["user2".to_string(), "user3".to_string()]),
        );

        // Test with LDAP as authority
        let changes_ldap = compute_group_sync_changes(
            defguard_memberships.clone(),
            ldap_memberships.clone(),
            Source::LDAP,
        );

        // group1: remove user2, add user4
        assert!(changes_ldap.delete_defguard.contains_key("group1"));
        assert_eq!(changes_ldap.delete_defguard["group1"].len(), 1);
        assert!(changes_ldap.delete_defguard["group1"].contains("user2"));
        assert!(changes_ldap.add_defguard.contains_key("group1"));
        assert_eq!(changes_ldap.add_defguard["group1"].len(), 1);
        assert!(changes_ldap.add_defguard["group1"].contains("user4"));

        // group2: remove user3, add user4
        assert!(changes_ldap.delete_defguard.contains_key("group2"));
        assert_eq!(changes_ldap.delete_defguard["group2"].len(), 1);
        assert!(changes_ldap.delete_defguard["group2"].contains("user3"));
        assert!(changes_ldap.add_defguard.contains_key("group2"));
        assert_eq!(changes_ldap.add_defguard["group2"].len(), 1);
        assert!(changes_ldap.add_defguard["group2"].contains("user4"));

        // group3: should be deleted entirely
        assert!(changes_ldap.delete_defguard.contains_key("group3"));
        assert_eq!(changes_ldap.delete_defguard["group3"].len(), 2);

        // group4: should be added entirely
        assert!(changes_ldap.add_defguard.contains_key("group4"));
        assert_eq!(changes_ldap.add_defguard["group4"].len(), 2);
        assert!(changes_ldap.add_defguard["group4"].contains("user2"));
        assert!(changes_ldap.add_defguard["group4"].contains("user3"));

        // Test with Defguard as authority
        let changes_defguard =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Source::Defguard);

        // group1: add user2, remove user4
        assert!(changes_defguard.add_ldap.contains_key("group1"));
        assert_eq!(changes_defguard.add_ldap["group1"].len(), 1);
        assert!(changes_defguard.add_ldap["group1"].contains("user2"));
        assert!(changes_defguard.delete_ldap.contains_key("group1"));
        assert_eq!(changes_defguard.delete_ldap["group1"].len(), 1);
        assert!(changes_defguard.delete_ldap["group1"].contains("user4"));

        // group2: add user3, remove user4
        assert!(changes_defguard.add_ldap.contains_key("group2"));
        assert_eq!(changes_defguard.add_ldap["group2"].len(), 1);
        assert!(changes_defguard.add_ldap["group2"].contains("user3"));
        assert!(changes_defguard.delete_ldap.contains_key("group2"));
        assert_eq!(changes_defguard.delete_ldap["group2"].len(), 1);
        assert!(changes_defguard.delete_ldap["group2"].contains("user4"));

        // group3: should be added entirely to ldap
        assert!(changes_defguard.add_ldap.contains_key("group3"));
        assert_eq!(changes_defguard.add_ldap["group3"].len(), 2);

        // group4: should be deleted entirely from ldap
        assert!(changes_defguard.delete_ldap.contains_key("group4"));
        assert_eq!(changes_defguard.delete_ldap["group4"].len(), 2);
    }

    #[test]
    fn test_extract_intersecting_users_empty() {
        let mut defguard_users = Vec::<User<Id>>::new();
        let mut ldap_users = Vec::<User>::new();

        let result = extract_intersecting_users(&mut defguard_users, &mut ldap_users);

        assert!(result.is_empty());
        assert!(defguard_users.is_empty());
        assert!(ldap_users.is_empty());
    }

    #[sqlx::test]
    fn test_extract_intersecting_users_with_matches(pool: PgPool) {
        // Create test users
        let user1 = User::new(
            "user1",
            Some("password"),
            "Last1",
            "First1",
            "user1@example.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let user2 = User::new(
            "user2",
            Some("password"),
            "Last2",
            "First2",
            "user2@example.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let user3 = User::new(
            "user3",
            Some("password"),
            "Last3",
            "First3",
            "user3@example.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        // Create LDAP users with same usernames
        let ldap_user1 = User::new(
            "user1",
            Some("ldap_password"),
            "LdapLast1",
            "LdapFirst1",
            "ldap_user1@example.com",
            None,
        );

        let ldap_user2 = User::new(
            "user2",
            Some("ldap_password"),
            "LdapLast2",
            "LdapFirst2",
            "ldap_user2@example.com",
            None,
        );

        let ldap_user4 = User::new(
            "user4",
            Some("ldap_password"),
            "LdapLast4",
            "LdapFirst4",
            "ldap_user4@example.com",
            None,
        );

        let mut defguard_users = vec![user1, user2, user3];
        let mut ldap_users = vec![ldap_user1, ldap_user2, ldap_user4];

        let result = extract_intersecting_users(&mut defguard_users, &mut ldap_users);

        // Should have 2 intersecting users (user1 and user2)
        assert_eq!(result.len(), 2);

        // Check usernames of matched pairs
        let usernames: Vec<(&str, &str)> = result
            .iter()
            .map(|(ldap, defguard)| (ldap.username.as_str(), defguard.username.as_str()))
            .collect();

        assert!(usernames.contains(&("user1", "user1")));
        assert!(usernames.contains(&("user2", "user2")));

        // Check remaining users
        assert_eq!(defguard_users.len(), 1);
        assert_eq!(defguard_users[0].username, "user3");

        assert_eq!(ldap_users.len(), 1);
        assert_eq!(ldap_users[0].username, "user4");
    }

    #[sqlx::test]
    fn test_extract_intersecting_users_no_matches(pool: PgPool) {
        let mut defguard_users = vec![User::new(
            "user1",
            Some("password"),
            "Last1",
            "First1",
            "user1@example.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap()];

        let mut ldap_users = vec![User::new(
            "user2",
            Some("password"),
            "Last",
            "First",
            "email@example.com",
            None,
        )];

        let result = extract_intersecting_users(&mut defguard_users, &mut ldap_users);

        assert!(result.is_empty());
        assert_eq!(defguard_users.len(), 1);
        assert_eq!(ldap_users.len(), 1);
    }
}
