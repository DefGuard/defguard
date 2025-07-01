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
pub(super) struct UserSyncChanges {
    pub delete_defguard: Vec<User<Id>>,
    pub add_defguard: Vec<User>,
    pub delete_ldap: Vec<User>,
    pub add_ldap: Vec<User<Id>>,
}

/// Computes what users should be added/deleted and where
pub(super) fn compute_user_sync_changes(
    all_ldap_users: &mut Vec<User>,
    all_defguard_users: &mut Vec<User<Id>>,
    authority: Authority,
    ldap_config: &LDAPConfig,
) -> UserSyncChanges {
    debug!("Computing user sync changes (user creation/deletion), authority: {authority:?}");
    let mut delete_defguard = Vec::new();
    let mut add_defguard = Vec::new();
    let mut delete_ldap = Vec::new();
    let mut add_ldap = Vec::new();

    let mut ldap_identifiers = HashSet::with_capacity(all_ldap_users.len());
    let defguard_identifiers = all_defguard_users
        .iter()
        .map(|u| ldap_config.user_dn_from_user(u))
        .collect::<HashSet<_>>();

    trace!("Defguard identifiers: {:?}", defguard_identifiers);
    trace!("LDAP identifiers: {:?}", ldap_identifiers);

    for user in all_ldap_users.drain(..) {
        ldap_identifiers.insert(ldap_config.user_dn_from_user(&user));

        debug!("Checking if user {} is in Defguard", user.username);
        if !defguard_identifiers.contains(&ldap_config.user_dn_from_user(&user)) {
            debug!("User {} not found in Defguard", user.username);
            match authority {
                Authority::LDAP => add_defguard.push(user),
                Authority::Defguard => delete_ldap.push(user),
            }
        }
    }

    for user in all_defguard_users.drain(..) {
        debug!("Checking if user {} is in LDAP", user.username);
        if !ldap_identifiers.contains(&ldap_config.user_dn_from_user(&user)) {
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
pub(super) struct GroupSyncChanges<'a> {
    pub add_defguard: HashMap<String, HashSet<&'a User>>,
    pub delete_defguard: HashMap<String, HashSet<User<Id>>>,
    pub add_ldap: HashMap<String, HashSet<User<Id>>>,
    pub delete_ldap: HashMap<String, HashSet<&'a User>>,
}

/// Computes what groups should be added/deleted and where
pub(super) fn compute_group_sync_changes<'a>(
    defguard_memberships: HashMap<String, HashSet<User<Id>>>,
    ldap_memberships: HashMap<String, HashSet<&'a User>>,
    authority: Authority,
    ldap_config: &LDAPConfig,
) -> GroupSyncChanges<'a> {
    debug!("Computing group sync changes (group membership changes), authority: {authority:?}");
    let mut delete_defguard = HashMap::new();
    let mut add_defguard = HashMap::new();
    let mut delete_ldap = HashMap::new();
    let mut add_ldap = HashMap::new();

    for (group, members) in defguard_memberships.clone() {
        debug!("Checking group {} for changes", group);
        if let Some(ldap_members) = ldap_memberships.get(&group) {
            debug!("Group {group:?} found in LDAP, checking for membership differences");
            let missing_from_defguard = ldap_members
                .iter()
                .filter(|u| {
                    !members.iter().any(|m| {
                        ldap_config.user_dn_from_user(m) == ldap_config.user_dn_from_user(u)
                    })
                })
                .cloned()
                .collect::<HashSet<_>>();

            let missing_from_ldap = members
                .iter()
                .filter(|m| {
                    !ldap_members.iter().any(|u| {
                        ldap_config.user_dn_from_user(m) == ldap_config.user_dn_from_user(u)
                    })
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
pub(super) fn extract_intersecting_users(
    defguard_users: &mut Vec<User<Id>>,
    ldap_users: &mut Vec<User>,
    ldap_config: &LDAPConfig,
) -> Vec<(User, User<Id>)> {
    let mut intersecting_users = vec![];
    let mut intersecting_users_ldap = vec![];

    for defguard_user in defguard_users.iter() {
        if let Some(ldap_user) = ldap_users
            .iter()
            .position(|u| {
                ldap_config.user_dn_from_user(u) == ldap_config.user_dn_from_user(defguard_user)
            })
            .map(|i| ldap_users.remove(i))
        {
            intersecting_users_ldap.push(ldap_user);
        }
    }

    for user in intersecting_users_ldap.into_iter() {
        if let Some(defguard_user) = defguard_users
            .iter()
            .position(|u| ldap_config.user_dn_from_user(u) == ldap_config.user_dn_from_user(&user))
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

        let user_dn = self.config.user_dn_from_user(user);
        let ldap_user = self.get_user_by_dn(user).await?;
        let defguard_groups = user.member_of_names(pool).await?;
        let mut ldap_groups = Vec::new();
        for group_entry in self.get_user_groups(&user_dn).await? {
            match self.group_entry_to_name(group_entry) {
                Ok(group_name) => ldap_groups.push(group_name),
                Err(err) => {
                    warn!(
                        "Failed to convert group entry to name during user synchronization: \
                        {err}. This group will be skipped"
                    );
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

        let changes = compute_group_sync_changes(
            defguard_memberships,
            ldap_memberships,
            authority,
            &self.config,
        );
        self.apply_user_group_sync_changes(pool, changes).await?;

        Ok(())
    }

    /// Fixes users with missing LDAP path
    /// This is for compatibility with older Defguard versions that didn't store LDAP paths in the database
    /// It will try to fetch the LDAP path from the LDAP server for users that have it missing
    /// If the user is not found in LDAP, it will skip fixing that user
    ///
    /// This function matches the user by username first, as those should be unique in both Defguard and LDAP.
    /// Then, just to make sure the user wasn't renamed, it checks if the RDN values match.
    pub(crate) async fn fix_missing_user_path(&mut self, pool: &PgPool) -> Result<(), LdapError> {
        debug!("Fixing missing user path in LDAP");

        let mut transaction = pool.begin().await?;
        let users = User::get_without_ldap_path(&mut *transaction).await?;

        let mut filtered_users = Vec::new();
        for user in users {
            if user.ldap_sync_allowed(&mut *transaction).await? {
                filtered_users.push(user);
            }
        }
        let users = filtered_users;

        for mut defguard_user in users {
            if defguard_user.ldap_user_path.is_none() {
                match self.get_user_by_username(&defguard_user).await {
                    Ok(ldap_user) => {
                        debug!(
                            "Found LDAP user {} with missing path in Defguard, fixing their path",
                            defguard_user.username
                        );
                        let defguard_user_rdn = defguard_user.ldap_rdn_value();
                        let ldap_user_rdn = ldap_user.ldap_rdn_value();

                        if defguard_user_rdn != ldap_user_rdn {
                            warn!(
                                "User {} has different RDN in Defguard ({}) and LDAP ({}), \
                                cannot fix missing LDAP path. Please update their username manually, so
                                they match in both sources.",
                                defguard_user.username, defguard_user_rdn, ldap_user_rdn
                            );
                            continue;
                        }

                        if let Some(ldap_path) = ldap_user.ldap_user_path {
                            debug!(
                                "Fixing the missing LDAP path of Defguard user {} to {}",
                                defguard_user.username, ldap_path
                            );
                            defguard_user.ldap_user_path = Some(ldap_path);
                            defguard_user.save(&mut *transaction).await?;
                        } else {
                            warn!(
                                "User {} has no LDAP path in LDAP, skipping fixing their path in Defguard",
                                defguard_user.username
                            );
                        }
                    }
                    Err(err) => {
                        debug!(
                            "Failed to get user {} from LDAP: {err}, cannot update their DN in Defguard",
                            defguard_user.username
                        );
                    }
                }
            }
        }

        transaction.commit().await?;
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

        self.fix_missing_user_path(pool).await?;

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
            extract_intersecting_users(&mut all_defguard_users, &mut all_ldap_users, &self.config);

        self.apply_user_modifications(intersecting_users, authority, pool)
            .await?;

        let user_changes = compute_user_sync_changes(
            &mut all_ldap_users,
            &mut all_defguard_users,
            authority,
            &self.config,
        );

        let membership_changes = compute_group_sync_changes(
            defguard_memberships,
            ldap_memberships,
            authority,
            &self.config,
        );

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
                            "Cannot remove last admin user {} from Defguard. User won't be removed \
                            from group {groupname}.",
                            member.username
                        );
                    } else {
                        debug!(
                            "Removing admin user {} from group {groupname}",
                            member.username
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
                        Your LDAP may have dangling group members. Skipping adding user to group \
                        {groupname}",
                        member.username
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
            if let Some(defguard_user) =
                User::find_by_username(&mut *transaction, &user.username).await?
            {
                let defguard_user_dn = self.config.user_dn_from_user(&defguard_user);
                let ldap_user_dn = self.config.user_dn_from_user(&user);
                if defguard_user_dn == ldap_user_dn {
                    debug!(
                        "User {} (DN: {}) already exists in Defguard, skipping...",
                        user.username, defguard_user_dn
                    );
                } else {
                    warn!(
                        "LDAP user with username {} already exists in Defguard. \
                        Those users have different DNs: {} (Defguard) vs {} (LDAP). \
                        All usernames must be unique, so this LDAP user will not be added to Defguard.",
                        user.username, ldap_user_dn, defguard_user_dn
                    );
                }
            } else {
                debug!(
                    "LDAP user {} does not exist in Defguard yet, adding...",
                    user.username
                );
                user.save(&mut *transaction).await?;
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

    pub(super) async fn get_all_users(&mut self) -> Result<Vec<User>, LdapError> {
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
                    LdapError::ObjectNotFound(format!("No {username_attr} attribute found"))
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
