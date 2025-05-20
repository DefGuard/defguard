//!
//! This module contains the logic for synchronizing users and groups between Defguard and LDAP.
//!
//! The synchronization is performed in two variants: full and incremental.
//!
//! # Sync status
//!
//! The sync status is stored in the database and can be either `InSync` or `OutOfSync`. The status is used to determine
//! whether the full sync should be performed or not. The status is set to `OutOfSync` when some Defguard changes
//! couldn't be propagated to LDAP (e.g. LDAP outage). The status is set to `InSync` when the sync is completed successfully.
//!
//! # Full synchronization
//!
//! The full synchronization takes all objects (users, groups and their memberships) from one source,
//! compares it with the other one and computes appropriate changes to make the two sources roughly equal.
//!
//! The full sync is performed only when the sync status is set to `OutOfSync`.
//!
//! The changes are computed with regard to a specified authority, which determines which source is considered to
//! be the more important one and which is expected to be edited more often. The authority can be either LDAP or Defguard.
//!
//! The authority has been introduced to solve the problem of ambiguity when some object is not present in one of the sources.
//! Such scenario may occur when a user is deleted from one of the sources OR when a user is added to one of the sources.
//! In each case, a different action should be taken to make the two sources equal (deletion or addition). For example:
//! - User is not present in LDAP but is present in Defguard
//! - Did we just add the user to Defguard but couldn't propagate that change or did we delete the user from LDAP?
//! - If the authority is LDAP, we should delete the user from Defguard, as we assume that it was more probable that the change was made in LDAP.
//! - If the authority is Defguard, we should add the user to LDAP, as we assume that it was more probable that the change was made in Defguard.
//!
//! If the LDAP connection is never lost and no other issues arise, the full sync should be performed only once, when the LDAP sync is enabled.
//! So this is a more of a damage control mechanism rather than something that should be invoked regularly.
//!
//! # Incremental synchronization
//!
//! The incremental synchronization is a regular synchronization operation which comes in two varieties: synchronous and asynchronous.
//!
//! Changes from Defguard are propagated to LDAP in real-time, synchronously, to keep LDAP up-to-date with Defguard instantly. This is done by
//! calling appropriate LDAP operations after each change in Defguard. Changes other way around (from LDAP to Defguard) are pulled asynchronously
//! at regular intervals (every 5 minutes by default). Implementation-wise it's done by running a full sync with LDAP authority, as it has the same effect
//! when we consider that LDAP has the most recent Defguard changes (due to synchronous change propagation).
//!
//! This synchronization should work reliably most of the time, given that:
//! - LDAP connection is stable
//! - The LDAP change pull is performed relatively often
//! - One object is not changed in both sources between two asynchronous syncs (may cause overwriting of changes), but this sounds like an unlikely scenario
//!
//! # Potential improvements and issues
//!
//! - Some optimizations could be made using the implementation-specific object modification/creation timestamps in LDAP. Currently everything is compared
//!   as is, without any regard to the time of the change. We could skip some operations on objects that haven't changed since the last sync. There is however
//!   still an issue with objects that have been deleted, LDAP doesn't store deleted objects by default, so we may still need to compare full object lists.
//! - There is no real pagination and everything is loaded into the memory at once. This may be an issue at some point. 10k LDAP records wasn't a problem in testing.
//!   We may have bigger issues with other parts of Defguard with that user count though.
//!
use std::collections::{HashMap, HashSet};

use sqlx::{PgConnection, PgPool, Type};

use super::{error::LdapError, LDAPConfig};
use crate::{
    db::{models::settings::update_current_settings, Group, Id, Settings, User},
    hashset,
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
pub enum Authority {
    LDAP,
    Defguard,
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Deserialize, Serialize, Default, Type)]
#[sqlx(type_name = "ldap_sync_status", rename_all = "lowercase")]
pub enum SyncStatus {
    InSync,
    #[default]
    OutOfSync,
}

impl SyncStatus {
    pub fn is_out_of_sync(&self) -> bool {
        matches!(self, SyncStatus::OutOfSync)
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

/// Computes what users should be added/deleted and where
fn compute_user_sync_changes(
    all_ldap_users: &mut Vec<User>,
    all_defguard_users: &mut Vec<User<Id>>,
    authority: Authority,
) -> UserSyncChanges {
    debug!("Computing user sync changes (user creation/deletion), authority: {authority:?}");
    let mut delete_defguard = Vec::new();
    let mut add_defguard = Vec::new();
    let mut delete_ldap = Vec::new();
    let mut add_ldap = Vec::new();

    let mut ldap_identifiers = HashSet::with_capacity(all_ldap_users.len());
    let defguard_identifiers = all_defguard_users
        .iter()
        .map(|u| u.ldap_rdn_value().to_string())
        .collect::<HashSet<_>>();

    trace!("Defguard identifiers: {:?}", defguard_identifiers);
    trace!("LDAP identifiers: {:?}", ldap_identifiers);

    for user in all_ldap_users.drain(..) {
        ldap_identifiers.insert(user.ldap_rdn_value().to_string());

        debug!("Checking if user {} is in Defguard", user.username);
        if !defguard_identifiers.contains(user.ldap_rdn_value()) {
            debug!("User {} not found in Defguard", user.username);
            match authority {
                Authority::LDAP => add_defguard.push(user),
                Authority::Defguard => delete_ldap.push(user),
            }
        }
    }

    for user in all_defguard_users.drain(..) {
        debug!("Checking if user {} is in LDAP", user.username);
        if !ldap_identifiers.contains(user.ldap_rdn_value()) {
            debug!("User {} not found in LDAP", user.username);
            match authority {
                Authority::LDAP => {
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
                Authority::Defguard => {
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
struct GroupSyncChanges<'a> {
    pub add_defguard: HashMap<String, HashSet<&'a User>>,
    pub delete_defguard: HashMap<String, HashSet<User<Id>>>,
    pub add_ldap: HashMap<String, HashSet<User<Id>>>,
    pub delete_ldap: HashMap<String, HashSet<&'a User>>,
}

/// Computes what groups should be added/deleted and where
fn compute_group_sync_changes(
    defguard_memberships: HashMap<String, HashSet<User<Id>>>,
    ldap_memberships: HashMap<String, HashSet<&User>>,
    authority: Authority,
) -> GroupSyncChanges<'_> {
    debug!("Computing group sync changes (group membership changes), authority: {authority:?}");
    let mut delete_defguard = HashMap::new();
    let mut add_defguard = HashMap::new();
    let mut delete_ldap = HashMap::new();
    let mut add_ldap = HashMap::new();

    // HashMap<groupname, HashMap<&user, ldap_rdn_value>>

    for (group, members) in defguard_memberships.clone() {
        debug!("Checking group {} for changes", group);
        if let Some(ldap_members) = ldap_memberships.get(&group) {
            debug!("Group {group:?} found in LDAP, checking for membership differences");
            let missing_from_defguard = ldap_members
                .iter()
                .filter(|u| {
                    !members
                        .iter()
                        .any(|m| m.ldap_rdn_value() == u.ldap_rdn_value())
                })
                .cloned()
                .collect::<HashSet<_>>();

            let missing_from_ldap = members
                .iter()
                .filter(|m| {
                    !ldap_members
                        .iter()
                        .any(|u| u.ldap_rdn_value() == m.ldap_rdn_value())
                })
                .cloned()
                .collect::<HashSet<_>>();

            trace!(
                "Group {group:?} members missing from Defguard: {missing_from_defguard:?}, missing from LDAP: {missing_from_ldap:?}"
            );

            if !missing_from_defguard.is_empty() {
                match authority {
                    Authority::Defguard => {
                        debug!("Group {group:?} has members missing from Defguard, marking them for deletion in LDAP: {missing_from_defguard:?}");
                        delete_ldap.insert(group.clone(), missing_from_defguard);
                    }
                    Authority::LDAP => {
                        debug!("Group {group:?} has members missing from Defguard, marking them for addition in Defguard: {missing_from_defguard:?}");
                        add_defguard.insert(group.clone(), missing_from_defguard);
                    }
                }
            } else {
                debug!("Group {group:?} has no members missing from Defguard");
            }

            if !missing_from_ldap.is_empty() {
                match authority {
                    Authority::Defguard => {
                        debug!("Group {group:?} has members missing from LDAP, marking them for addition to LDAP: {missing_from_ldap:?}");
                        add_ldap.insert(group.clone(), missing_from_ldap);
                    }
                    Authority::LDAP => {
                        debug!("Group {group:?} has members missing from LDAP, marking them for deletion in Defguard: {missing_from_ldap:?}");
                        delete_defguard.insert(group.clone(), missing_from_ldap);
                    }
                }
            } else {
                debug!("Group {group:?} has no members missing from LDAP");
            }
        } else {
            match authority {
                Authority::Defguard => {
                    debug!("Group {group:?} is missing from LDAP, marking it for addition to LDAP along with all members due to Defguard authority");
                    add_ldap.insert(group.clone(), members);
                }
                Authority::LDAP => {
                    debug!("Group {group:?} is missing from LDAP, marking all its member for deletion from Defguard due to LDAP authority");
                    delete_defguard.insert(group.clone(), members);
                }
            }
        }
    }

    for (group, members) in ldap_memberships {
        if !defguard_memberships.contains_key(&group) {
            match authority {
                Authority::Defguard => {
                    debug!("Group {group:?} is missing from Defguard, marking all its member for deletion from LDAP due to Defguard authority");
                    delete_ldap.insert(group, members);
                }
                Authority::LDAP => {
                    debug!("Group {group:?} is missing from Defguard, marking all its member for addition to Defguard due to LDAP authority");
                    add_defguard.insert(group, members);
                }
            }
        }
    }

    let sync_changes = GroupSyncChanges {
        add_defguard,
        delete_defguard,
        add_ldap,
        delete_ldap,
    };

    debug!("Completed computing group sync changes");
    trace!("Group sync changes: {sync_changes:?}");

    sync_changes
}

fn attrs_different(defguard_user: &User<Id>, ldap_user: &User, config: &LDAPConfig) -> bool {
    let mut different = false;

    if defguard_user.last_name != ldap_user.last_name {
        debug!(
            "Attribute difference detected: last_name (Defguard: {}, LDAP: {})",
            defguard_user.last_name, ldap_user.last_name
        );
        different = true;
    }

    if defguard_user.first_name != ldap_user.first_name {
        debug!(
            "Attribute difference detected: first_name (Defguard: {}, LDAP: {})",
            defguard_user.first_name, ldap_user.first_name
        );
        different = true;
    }

    if defguard_user.email != ldap_user.email {
        debug!(
            "Attribute difference detected: email (Defguard: {}, LDAP: {})",
            defguard_user.email, ldap_user.email
        );
        different = true;
    }

    if defguard_user.phone != ldap_user.phone
        && !(defguard_user.phone.as_deref() == Some("") && ldap_user.phone.is_none())
        && !(ldap_user.phone.as_deref() == Some("") && defguard_user.phone.is_none())
    {
        debug!(
            "Attribute difference detected: phone (Defguard: {:?}, LDAP: {:?})",
            defguard_user.phone, ldap_user.phone
        );
        different = true;
    }

    if !config.using_username_as_rdn() && defguard_user.username != ldap_user.username {
        debug!(
            "Attribute difference detected: username (Defguard: {}, LDAP: {})",
            defguard_user.username, ldap_user.username
        );
        different = true;
    }

    different
}

/// Extracts users that are in both sources for later comparison and attritubte modification (emails, phone numbers)
fn extract_intersecting_users(
    defguard_users: &mut Vec<User<Id>>,
    ldap_users: &mut Vec<User>,
) -> Vec<(User, User<Id>)> {
    let mut intersecting_users = vec![];
    let mut intersecting_users_ldap = vec![];

    for defguard_user in defguard_users.iter() {
        if let Some(ldap_user) = ldap_users
            .iter()
            .position(|u| u.ldap_rdn_value() == defguard_user.ldap_rdn_value())
            .map(|i| ldap_users.remove(i))
        {
            intersecting_users_ldap.push(ldap_user);
        }
    }

    for user in intersecting_users_ldap.into_iter() {
        if let Some(defguard_user) = defguard_users
            .iter()
            .position(|u| u.ldap_rdn_value() == user.ldap_rdn_value())
            .map(|i| defguard_users.remove(i))
        {
            intersecting_users.push((user, defguard_user));
        }
    }

    intersecting_users
}

const DEFAULT_LDAP_SYNC_INTERVAL: u64 = 60 * 5;

pub fn get_ldap_sync_interval() -> u64 {
    let settings = Settings::get_current_settings();
    settings
        .ldap_sync_interval
        .try_into()
        .unwrap_or(DEFAULT_LDAP_SYNC_INTERVAL)
}

impl super::LDAPConnection {
    /// Applies user modifications to users that are present in both LDAP and Defguard
    async fn apply_user_modifications(
        &mut self,
        mut intersecting_users: Vec<(User, User<Id>)>,
        authority: Authority,
        pool: &PgPool,
    ) -> Result<(), LdapError> {
        let mut transaction = pool.begin().await?;

        for (ldap_user, defguard_user) in intersecting_users.iter_mut() {
            if attrs_different(defguard_user, ldap_user, &self.config) {
                debug!(
                    "User {defguard_user} attributes differ between LDAP and Defguard, merging..."
                );
                match authority {
                    Authority::LDAP => {
                        debug!("Applying LDAP user attributes to Defguard user");
                        defguard_user.update_from_ldap_user(ldap_user, &self.config);
                        defguard_user.save(&mut *transaction).await?;
                    }
                    Authority::Defguard => {
                        debug!("Applying Defguard user attributes to LDAP user");
                        self.modify_user(&ldap_user.username, defguard_user).await?;
                    }
                }
            }
        }

        transaction.commit().await?;

        Ok(())
    }

    /// Allows to synchronize user data (e.g. email, groups) between Defguard and LDAP based on the authority for a single user
    ///
    /// Does nothing if the two way sync is disabled
    pub(crate) async fn sync_user_data(
        &mut self,
        user: &User<Id>,
        pool: &PgPool,
    ) -> Result<(), LdapError> {
        debug!("Syncing user data for {user}");
        let settings = Settings::get_current_settings();

        // Force update user data in LDAP if the two-way sync is disabled, otherwise respect the authority
        let authority = if !settings.ldap_sync_enabled || !settings.ldap_is_authoritative {
            Authority::Defguard
        } else {
            Authority::LDAP
        };

        let user_dn = self.config.user_dn(user.ldap_rdn_value());
        let ldap_user = self.get_user(user).await?;
        let defguard_groups = user.member_of_names(pool).await?;
        let mut ldap_groups = Vec::new();
        for group_entry in self.get_user_groups(&user_dn).await? {
            match self.group_entry_to_name(group_entry) {
                Ok(group_name) => ldap_groups.push(group_name),
                Err(err) => {
                    warn!("Failed to convert group entry to name during user synchronization: {err}. This group will be skipped");
                    continue;
                }
            }
        }

        debug!("User {user} is a member of the following groups in Defguard: {defguard_groups:?}");
        debug!("User {user} is a member of the following groups in LDAP: {ldap_groups:?}");

        let intersecting_users = vec![(ldap_user.clone(), user.clone())];

        // create a hashmap for the calculate group membership changes function
        let defguard_memberships = defguard_groups
            .iter()
            .map(|g| (g.clone(), hashset![user.clone()]))
            .collect::<HashMap<_, _>>();

        let ldap_memberships = ldap_groups
            .iter()
            .map(|g| (g.clone(), hashset![&ldap_user]))
            .collect::<HashMap<_, _>>();

        self.apply_user_modifications(intersecting_users, authority, pool)
            .await?;

        let changes = compute_group_sync_changes(defguard_memberships, ldap_memberships, authority);
        self.apply_user_group_sync_changes(pool, changes).await?;

        Ok(())
    }

    /// Synchronizes users and groups between Defguard and LDAP
    pub(crate) async fn sync(&mut self, pool: &PgPool, full: bool) -> Result<(), LdapError> {
        let settings = Settings::get_current_settings();
        let authority = if full {
            let settings_authority = if settings.ldap_is_authoritative {
                Authority::LDAP
            } else {
                Authority::Defguard
            };
            debug!(
                "Full LDAP sync requested, using the following authority: {settings_authority:?}"
            );
            settings_authority
        } else {
            debug!("Incremental LDAP sync requested.");
            Authority::LDAP
        };

        let mut sync_groups = Vec::new();
        for groupname in &self.config.ldap_sync_groups {
            if let Some(group) = Group::find_by_name(pool, groupname).await? {
                sync_groups.push(group);
            } else {
                debug!("Group {groupname} not found in Defguard, skipping");
            }
        }

        debug!("The following groups were defined for sync: {:?}, only Defguard users belonging to these groups will be synced", sync_groups);
        let mut sync_group_members = HashSet::new();
        for sync_group in &sync_groups {
            let members = sync_group.members(pool).await?;
            sync_group_members.extend(members.into_iter());
        }

        let mut all_ldap_users = self.get_all_users().await?;
        let mut all_defguard_users = User::all(pool).await?;

        // Filter out users that should be ignored from sync
        let mut filtered_users = Vec::new();
        for user in all_defguard_users {
            if user.ldap_sync_allowed(pool).await? {
                filtered_users.push(user);
            }
        }
        all_defguard_users = filtered_users;

        let ldap_usernames = all_ldap_users
            .iter()
            .map(|u| u.username.as_str())
            .collect::<HashSet<_>>();
        let defguard_usernames = all_defguard_users
            .iter()
            .map(|u| u.username.as_str())
            .collect::<HashSet<_>>();

        debug!("LDAP users: {:?}", ldap_usernames);
        debug!("Defguard users: {:?}", defguard_usernames);

        let all_ldap_users_groupsync = all_ldap_users.clone();
        let ldap_memberships = self
            .get_ldap_group_memberships(&all_ldap_users_groupsync)
            .await?;
        let mut defguard_memberships = HashMap::new();
        let defguard_groups = Group::all(pool).await?;

        for group in defguard_groups {
            let mut members = HashSet::new();
            for member in group.members(pool).await? {
                if member.ldap_sync_allowed(pool).await? {
                    members.insert(member);
                }
            }
            defguard_memberships.insert(group.name, members);
        }

        let intersecting_users =
            extract_intersecting_users(&mut all_defguard_users, &mut all_ldap_users);
        self.apply_user_modifications(intersecting_users, authority, pool)
            .await?;

        let user_changes =
            compute_user_sync_changes(&mut all_ldap_users, &mut all_defguard_users, authority);

        debug!("Defguard group memberships: {:?}", defguard_memberships);
        debug!("LDAP group memberships: {:?}", ldap_memberships);

        let membership_changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, authority);

        debug!("Membership changes: {:?}", membership_changes);

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
        changes: GroupSyncChanges<'_>,
    ) -> Result<(), LdapError> {
        debug!("Applying group memberships sync changes");
        let mut transaction = pool.begin().await?;
        let mut admin_count = User::find_admins(&mut *transaction).await?.len();
        for (groupname, members) in changes.delete_defguard {
            if members.is_empty() {
                debug!("No members to remove from group {groupname}, skipping");
                continue;
            }
            let group = get_or_create_group(&mut transaction, &groupname).await?;

            for member in members {
                if member.is_admin(&mut *transaction).await? {
                    if admin_count == 1 {
                        debug!(
                            "Cannot remove last admin user {} from Defguard. User won't be removed from group {}.",
                            member.username, groupname
                        );
                        continue;
                    } else {
                        debug!(
                            "Removing admin user {} from group {}",
                            member.username, groupname
                        );
                        admin_count -= 1;
                        member.remove_from_group(&mut *transaction, &group).await?;
                    }
                } else {
                    debug!("Removing user {} from group {}", member.username, groupname);
                    member.remove_from_group(&mut *transaction, &group).await?;
                }
            }
        }

        for (groupname, members) in changes.add_defguard {
            if members.is_empty() {
                debug!("No members to add to group {groupname}, skipping");
                continue;
            }
            let group = get_or_create_group(&mut transaction, &groupname).await?;
            for member in members {
                if let Some(user) =
                    User::find_by_username(&mut *transaction, &member.username).await?
                {
                    user.add_to_group(&mut *transaction, &group).await?;
                } else {
                    warn!(
                        "LDAP user {} not found in Defguard, despite completing user sync earlier. \
                        Your LDAP may have dangling group members. Skipping adding user to group {}",
                        member.username, groupname
                    );
                }
            }
        }

        transaction.commit().await?;

        for (groupname, members) in changes.delete_ldap {
            for member in members {
                self.remove_user_from_group(member, &groupname).await?;
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
        mut changes: UserSyncChanges,
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
            // check if the user doesnt exist in defguard
            if User::find_by_username(&mut *transaction, &user.username)
                .await?
                .is_none()
            {
                debug!(
                    "LDAP user {} does not exist in Defguard yet, adding...",
                    user.username
                );
                user.save(&mut *transaction).await?;
            } else {
                debug!(
                    "LDAP user {} already exists in Defguard, skipping",
                    user.username
                );
            }
        }

        transaction.commit().await?;

        for user in changes.delete_ldap {
            debug!("Deleting user {} from LDAP", user.username);
            self.delete_user(&user).await?;
        }

        for user in changes.add_ldap.iter_mut() {
            debug!("Adding user {} to LDAP", user.username);
            self.add_user(user, None, pool).await?;
        }

        Ok(())
    }

    async fn get_all_users(&mut self) -> Result<Vec<User>, LdapError> {
        debug!("Retrieving all LDAP users");
        let all_ldap_user_entries = self.list_users().await?;
        let mut all_users = Vec::new();
        let username_attr = &self.config.ldap_username_attr;

        for entry in all_ldap_user_entries {
            let username = entry
                .attrs
                .get(username_attr)
                .and_then(|v| v.first())
                .ok_or_else(|| {
                    LdapError::ObjectNotFound(format!("No {} attribute found", username_attr))
                })?;

            match User::from_searchentry(&entry, username, None) {
                Ok(user) => all_users.push(user),
                Err(err) => {
                    warn!(
                        "Failed to create user {} from LDAP entry, error: {}. The user will be skipped during sync",
                        username, err
                    );
                    debug!("Skipping user {} due to error: {}", username, err);
                }
            }
        }

        Ok(all_users)
    }
}

#[cfg(test)]
mod tests {
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

    use super::*;
    use crate::db::setup_pool;

    fn make_test_user(username: &str) -> User {
        User::new(
            username,
            Some("test_password"),
            "last name",
            "first name",
            format!("{}@example.com", username).as_str(),
            None,
        )
    }

    #[test]
    fn test_compute_user_sync_changes_empty_lists() {
        let mut ldap_users: Vec<User> = vec![];
        let mut defguard_users: Vec<User<Id>> = vec![];

        let changes =
            compute_user_sync_changes(&mut ldap_users, &mut defguard_users, Authority::LDAP);

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

        let mut ldap_users = vec![ldap_user];
        let mut defguard_users: Vec<User<Id>> = vec![];

        let changes =
            compute_user_sync_changes(&mut ldap_users, &mut defguard_users, Authority::LDAP);

        assert!(changes.delete_defguard.is_empty());
        assert_eq!(changes.add_defguard.len(), 1);
        assert_eq!(changes.add_defguard[0].username, "test_user");
        assert!(changes.delete_ldap.is_empty());
        assert!(changes.add_ldap.is_empty());
    }

    #[sqlx::test]
    fn test_ldap_authority_delete_from_defguard(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

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

        let mut ldap_users: Vec<User> = vec![];
        let mut defguard_users = vec![defguard_user];

        let changes =
            compute_user_sync_changes(&mut ldap_users, &mut defguard_users, Authority::LDAP);

        assert_eq!(changes.delete_defguard.len(), 1);
        assert_eq!(changes.delete_defguard[0].username, "test_user");
        assert!(changes.add_defguard.is_empty());
        assert!(changes.delete_ldap.is_empty());
        assert!(changes.add_ldap.is_empty());
    }

    #[sqlx::test]
    fn test_defguard_authority_add_to_ldap(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

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

        let mut ldap_users: Vec<User> = vec![];
        let mut defguard_users = vec![defguard_user];

        let changes =
            compute_user_sync_changes(&mut ldap_users, &mut defguard_users, Authority::Defguard);

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

        let mut ldap_users = vec![ldap_user];
        let mut defguard_users: Vec<User<Id>> = vec![];

        let changes =
            compute_user_sync_changes(&mut ldap_users, &mut defguard_users, Authority::Defguard);

        assert!(changes.delete_defguard.is_empty());
        assert!(changes.add_defguard.is_empty());
        assert_eq!(changes.delete_ldap.len(), 1);
        assert_eq!(changes.delete_ldap[0].username, "test_user");
        assert!(changes.add_ldap.is_empty());
    }

    #[sqlx::test]
    fn test_matching_users_no_changes(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

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

        let mut ldap_users = vec![ldap_user];
        let mut defguard_users = vec![defguard_user];

        let changes_ldap = compute_user_sync_changes(
            &mut ldap_users.clone(),
            &mut defguard_users.clone(),
            Authority::LDAP,
        );

        assert!(changes_ldap.delete_defguard.is_empty());
        assert!(changes_ldap.add_defguard.is_empty());
        assert!(changes_ldap.delete_ldap.is_empty());
        assert!(changes_ldap.add_ldap.is_empty());

        let changes_defguard =
            compute_user_sync_changes(&mut ldap_users, &mut defguard_users, Authority::Defguard);

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
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Authority::LDAP);

        assert!(changes.delete_defguard.is_empty());
        assert!(changes.add_defguard.is_empty());
        assert!(changes.delete_ldap.is_empty());
        assert!(changes.add_ldap.is_empty());
    }

    #[test]
    fn test_ldap_authority_add_group_to_defguard() {
        let defguard_memberships = HashMap::new();
        let mut ldap_memberships = HashMap::new();
        let test_user = make_test_user("user1");
        ldap_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec![&test_user]),
        );

        let changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Authority::LDAP);

        assert!(changes.delete_defguard.is_empty());
        assert_eq!(changes.add_defguard.len(), 1);
        assert!(changes.add_defguard.contains_key("test_group"));
        assert_eq!(changes.add_defguard["test_group"].len(), 1);
        assert!(changes.add_defguard["test_group"].contains(&test_user));
        assert!(changes.delete_ldap.is_empty());
        assert!(changes.add_ldap.is_empty());
    }

    #[sqlx::test]
    fn test_ldap_authority_delete_group_from_defguard(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let mut defguard_memberships = HashMap::new();
        let test_user = make_test_user("user1").save(&pool).await.unwrap();
        defguard_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec![test_user.clone()]),
        );
        let ldap_memberships = HashMap::new();

        let changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Authority::LDAP);

        assert_eq!(changes.delete_defguard.len(), 1);
        assert!(changes.delete_defguard.contains_key("test_group"));
        assert_eq!(changes.delete_defguard["test_group"].len(), 1);
        assert!(changes.delete_defguard["test_group"].contains(&test_user));
        assert!(changes.add_defguard.is_empty());
        assert!(changes.delete_ldap.is_empty());
        assert!(changes.add_ldap.is_empty());
    }

    #[sqlx::test]
    fn test_defguard_authority_add_group_to_ldap(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let mut defguard_memberships = HashMap::new();
        let test_user = make_test_user("user1").save(&pool).await.unwrap();
        defguard_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec![test_user.clone()]),
        );
        let ldap_memberships = HashMap::new();

        let changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Authority::Defguard);

        assert!(changes.delete_defguard.is_empty());
        assert!(changes.add_defguard.is_empty());
        assert!(changes.delete_ldap.is_empty());
        assert_eq!(changes.add_ldap.len(), 1);
        assert!(changes.add_ldap.contains_key("test_group"));
        assert_eq!(changes.add_ldap["test_group"].len(), 1);
        assert!(changes.add_ldap["test_group"].contains(&test_user));
    }

    #[test]
    fn test_defguard_authority_delete_group_from_ldap() {
        let defguard_memberships = HashMap::new();
        let mut ldap_memberships = HashMap::new();
        let test_user = make_test_user("user1");
        ldap_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec![&test_user]),
        );

        let changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Authority::Defguard);

        assert!(changes.delete_defguard.is_empty());
        assert!(changes.add_defguard.is_empty());
        assert_eq!(changes.delete_ldap.len(), 1);
        assert!(changes.delete_ldap.contains_key("test_group"));
        assert_eq!(changes.delete_ldap["test_group"].len(), 1);
        assert!(changes.delete_ldap["test_group"].contains(&test_user));
        assert!(changes.add_ldap.is_empty());
    }

    #[sqlx::test]
    fn test_matching_groups_no_changes(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let mut defguard_memberships = HashMap::new();
        let test_user = make_test_user("user1");
        let test_user_id = test_user.clone().save(&pool).await.unwrap();
        defguard_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec![test_user_id]),
        );
        let mut ldap_memberships = HashMap::new();
        ldap_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec![&test_user]),
        );

        let changes_ldap = compute_group_sync_changes(
            defguard_memberships.clone(),
            ldap_memberships.clone(),
            Authority::LDAP,
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
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Authority::Defguard);

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

    #[sqlx::test]
    fn test_ldap_authority_add_users_to_group(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let test_user = make_test_user("user1");
        let test_user_id = test_user.clone().save(&pool).await.unwrap();
        let test_user2 = make_test_user("user2");
        let mut defguard_memberships = HashMap::new();
        defguard_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec![test_user_id]),
        );
        let mut ldap_memberships = HashMap::new();
        ldap_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec![&test_user, &test_user2]),
        );

        let changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Authority::LDAP);

        assert!(changes.add_defguard.contains_key("test_group"));
        assert_eq!(changes.add_defguard["test_group"].len(), 1);
        assert!(changes.add_defguard["test_group"].contains(&test_user2));
    }

    #[sqlx::test]
    fn test_ldap_authority_remove_users_from_group(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let mut defguard_memberships = HashMap::new();
        let user1 = make_test_user("user1").save(&pool).await.unwrap();
        let user2 = make_test_user("user2").save(&pool).await.unwrap();
        let user1_noid = user1.clone().as_noid();
        defguard_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec![user1, user2.clone()]),
        );
        let mut ldap_memberships = HashMap::new();
        ldap_memberships.insert(
            "test_group".to_string(),
            HashSet::from_iter(vec![&user1_noid]),
        );

        let changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Authority::LDAP);

        assert!(changes.delete_defguard.contains_key("test_group"));
        assert_eq!(changes.delete_defguard["test_group"].len(), 1);
        assert!(changes.delete_defguard["test_group"].contains(&user2));
    }

    #[sqlx::test]
    fn test_multiple_groups_ldap_authority(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let user1 = make_test_user("user1").save(&pool).await.unwrap();
        let user2 = make_test_user("user2").save(&pool).await.unwrap();
        let user3 = make_test_user("user3").save(&pool).await.unwrap();
        let user4 = make_test_user("user4");
        let user5 = make_test_user("user5");
        let user6 = make_test_user("user6");
        let user1_noid = user1.clone().as_noid();
        let mut defguard_memberships = HashMap::new();
        defguard_memberships.insert(
            "group1".to_string(),
            HashSet::from_iter(vec![user1.clone(), user2.clone()]),
        );
        defguard_memberships.insert(
            "group2".to_string(),
            HashSet::from_iter(vec![user3.clone()]),
        );

        let mut ldap_memberships = HashMap::new();
        ldap_memberships.insert(
            "group1".to_string(),
            HashSet::from_iter(vec![&user1_noid, &user4]),
        );
        ldap_memberships.insert(
            "group3".to_string(),
            HashSet::from_iter(vec![&user5, &user6]),
        );

        let changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Authority::LDAP);

        // group1: remove user2, add user4
        assert!(changes.delete_defguard.contains_key("group1"));
        assert_eq!(changes.delete_defguard["group1"].len(), 1);
        assert!(changes.delete_defguard["group1"].contains(&user2));
        assert!(changes.add_defguard.contains_key("group1"));
        assert_eq!(changes.add_defguard["group1"].len(), 1);
        assert!(changes.add_defguard["group1"].contains(&user4));

        // group2: should be deleted entirely
        assert!(changes.delete_defguard.contains_key("group2"));
        assert_eq!(changes.delete_defguard["group2"].len(), 1);
        assert!(changes.delete_defguard["group2"].contains(&user3));

        // group3: should be added entirely
        assert!(changes.add_defguard.contains_key("group3"));
        assert_eq!(changes.add_defguard["group3"].len(), 2);
        assert!(changes.add_defguard["group3"].contains(&user5));
        assert!(changes.add_defguard["group3"].contains(&user6));

        // Nothing should be changed in LDAP since we use LDAP as authority
        assert!(changes.delete_ldap.is_empty());
        assert!(changes.add_ldap.is_empty());
    }

    #[sqlx::test]
    fn test_multiple_groups_defguard_authority(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let user1 = make_test_user("user1").save(&pool).await.unwrap();
        let user2 = make_test_user("user2").save(&pool).await.unwrap();
        let user5 = make_test_user("user5").save(&pool).await.unwrap();
        let user6 = make_test_user("user6").save(&pool).await.unwrap();
        let user1_noid = user1.clone().as_noid();
        let user4 = make_test_user("user4");
        let user3 = make_test_user("user3");
        let mut defguard_memberships = HashMap::new();
        defguard_memberships.insert(
            "group1".to_string(),
            HashSet::from_iter(vec![user1.clone(), user2.clone()]),
        );
        defguard_memberships.insert(
            "group3".to_string(),
            HashSet::from_iter(vec![user5.clone(), user6.clone()]),
        );

        let mut ldap_memberships = HashMap::new();
        ldap_memberships.insert(
            "group1".to_string(),
            HashSet::from_iter(vec![&user1_noid, &user4]),
        );
        ldap_memberships.insert("group2".to_string(), HashSet::from_iter(vec![&user3]));

        let changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Authority::Defguard);

        assert!(changes.delete_defguard.is_empty());
        assert!(changes.add_defguard.is_empty());

        // group1: add user2, remove user4
        assert!(changes.add_ldap.contains_key("group1"));
        assert_eq!(changes.add_ldap["group1"].len(), 1);
        assert!(changes.add_ldap["group1"].contains(&user2));
        assert!(changes.delete_ldap.contains_key("group1"));
        assert_eq!(changes.delete_ldap["group1"].len(), 1);
        assert!(changes.delete_ldap["group1"].contains(&user4));

        // group2: should be deleted entirely
        assert!(changes.delete_ldap.contains_key("group2"));
        assert_eq!(changes.delete_ldap["group2"].len(), 1);
        assert!(changes.delete_ldap["group2"].contains(&user3));

        // group3: should be added entirely to LDAP
        assert!(changes.add_ldap.contains_key("group3"));
        assert_eq!(changes.add_ldap["group3"].len(), 2);
        assert!(changes.add_ldap["group3"].contains(&user5));
        assert!(changes.add_ldap["group3"].contains(&user6));
    }

    #[test]
    fn test_empty_groups() {
        let mut defguard_memberships = HashMap::new();
        defguard_memberships.insert("empty_group1".to_string(), HashSet::new());

        let mut ldap_memberships = HashMap::new();
        ldap_memberships.insert("empty_group2".to_string(), HashSet::new());

        let changes =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Authority::LDAP);

        // empty_group1 should be deleted from defguard (it's not in LDAP)
        assert!(changes.delete_defguard.contains_key("empty_group1"));
        assert_eq!(changes.delete_defguard["empty_group1"].len(), 0);
        assert!(changes.delete_defguard["empty_group1"].is_empty());

        // empty_group2 should be added to defguard (it's in LDAP)
        assert!(changes.add_defguard.contains_key("empty_group2"));
        assert_eq!(changes.add_defguard["empty_group2"].len(), 0);
        assert!(changes.add_defguard["empty_group2"].is_empty());
    }

    #[sqlx::test]
    fn test_complex_group_memberships(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let user1 = make_test_user("user1").save(&pool).await.unwrap();
        let user2 = make_test_user("user2").save(&pool).await.unwrap();
        let user3 = make_test_user("user3").save(&pool).await.unwrap();
        let user4 = make_test_user("user4");
        let user5 = make_test_user("user5").save(&pool).await.unwrap();
        let user1_noid = user1.clone().as_noid();
        let user2_noid = user2.clone().as_noid();
        let user3_noid = user3.clone().as_noid();

        let mut defguard_memberships = HashMap::new();
        defguard_memberships.insert(
            "group1".to_string(),
            HashSet::from_iter(vec![user1.clone(), user2.clone()]),
        );
        defguard_memberships.insert(
            "group2".to_string(),
            HashSet::from_iter(vec![user1.clone(), user2.clone(), user3.clone()]),
        );
        defguard_memberships.insert(
            "group3".to_string(),
            HashSet::from_iter(vec![user1.clone(), user5.clone()]),
        );

        let mut ldap_memberships = HashMap::new();
        ldap_memberships.insert(
            "group1".to_string(),
            HashSet::from_iter(vec![&user1_noid, &user4]),
        );
        ldap_memberships.insert(
            "group2".to_string(),
            HashSet::from_iter(vec![&user1_noid, &user2_noid, &user4]),
        );
        ldap_memberships.insert(
            "group4".to_string(),
            HashSet::from_iter(vec![&user2_noid, &user3_noid]),
        );

        // Test with LDAP as authority
        let changes_ldap = compute_group_sync_changes(
            defguard_memberships.clone(),
            ldap_memberships.clone(),
            Authority::LDAP,
        );

        // group1: remove user2, add user4
        assert!(changes_ldap.delete_defguard.contains_key("group1"));
        assert_eq!(changes_ldap.delete_defguard["group1"].len(), 1);
        assert!(changes_ldap.delete_defguard["group1"].contains(&user2));
        assert!(changes_ldap.add_defguard.contains_key("group1"));
        assert_eq!(changes_ldap.add_defguard["group1"].len(), 1);
        assert!(changes_ldap.add_defguard["group1"].contains(&user4));

        // group2: remove user3, add user4
        assert!(changes_ldap.delete_defguard.contains_key("group2"));
        assert_eq!(changes_ldap.delete_defguard["group2"].len(), 1);
        assert!(changes_ldap.delete_defguard["group2"].contains(&user3));
        assert!(changes_ldap.add_defguard.contains_key("group2"));
        assert_eq!(changes_ldap.add_defguard["group2"].len(), 1);
        assert!(changes_ldap.add_defguard["group2"].contains(&user4));

        // group3: should be deleted entirely
        assert!(changes_ldap.delete_defguard.contains_key("group3"));
        assert_eq!(changes_ldap.delete_defguard["group3"].len(), 2);

        // group4: should be added entirely
        assert!(changes_ldap.add_defguard.contains_key("group4"));
        assert_eq!(changes_ldap.add_defguard["group4"].len(), 2);
        assert!(changes_ldap.add_defguard["group4"].contains(&user2_noid));
        assert!(changes_ldap.add_defguard["group4"].contains(&user3_noid));

        // Test with Defguard as authority
        let changes_defguard =
            compute_group_sync_changes(defguard_memberships, ldap_memberships, Authority::Defguard);

        // group1: add user2, remove user4
        assert!(changes_defguard.add_ldap.contains_key("group1"));
        assert_eq!(changes_defguard.add_ldap["group1"].len(), 1);
        assert!(changes_defguard.add_ldap["group1"].contains(&user2));
        assert!(changes_defguard.delete_ldap.contains_key("group1"));
        assert_eq!(changes_defguard.delete_ldap["group1"].len(), 1);
        assert!(changes_defguard.delete_ldap["group1"].contains(&user4));

        // group2: add user3, remove user4
        assert!(changes_defguard.add_ldap.contains_key("group2"));
        assert_eq!(changes_defguard.add_ldap["group2"].len(), 1);
        assert!(changes_defguard.add_ldap["group2"].contains(&user3));
        assert!(changes_defguard.delete_ldap.contains_key("group2"));
        assert_eq!(changes_defguard.delete_ldap["group2"].len(), 1);
        assert!(changes_defguard.delete_ldap["group2"].contains(&user4));

        // group3: should be added entirely to ldap
        assert!(changes_defguard.add_ldap.contains_key("group3"));
        assert_eq!(changes_defguard.add_ldap["group3"].len(), 2);

        // group4: should be deleted entirely from ldap
        assert!(changes_defguard.delete_ldap.contains_key("group4"));
        assert_eq!(changes_defguard.delete_ldap["group4"].len(), 2);
        assert!(changes_defguard.delete_ldap["group4"].contains(&user2_noid));
        assert!(changes_defguard.delete_ldap["group4"].contains(&user3_noid));
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
    fn test_extract_intersecting_users_with_matches(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

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
    fn test_extract_intersecting_users_no_matches(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

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
