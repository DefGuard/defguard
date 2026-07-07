//!
//! This module contains the logic for synchronizing users and groups between Defguard and LDAP.
//!
//! The synchronization is performed in two variants: full and incremental.
//!
//! # Sync status
//!
//! The sync status is stored in the database and can be either `InSync` or `OutOfSync`. The status
//! is used to determine whether the full sync should be performed or not. The status is set to
//! `OutOfSync` when some Defguard changes couldn't be propagated to LDAP (e.g. LDAP outage). The
//! status is set to `InSync` when the sync is completed successfully.
//!
//! # Full synchronization
//!
//! The full synchronization takes all objects (users, groups and their memberships) from one
//! source, compares it with the other one and computes appropriate changes to make the two sources
//! roughly equal.
//!
//! The full sync is performed only when the sync status is set to `OutOfSync`.
//!
//! The changes are computed with regard to a specified authority, which determines which source is
//! considered to be the more important one and which is expected to be edited more often. The
//! authority can be either LDAP or Defguard.
//!
//! The authority has been introduced to solve the problem of ambiguity when some object is not
//! present in one of the sources. Such scenario may occur when a user is deleted from one of the
//! sources OR when a user is added to one of the sources. In each case, a different action should
//! be taken to make the two sources equal (deletion or addition). For example:
//! - User is not present in LDAP but is present in Defguard
//! - Did we just add the user to Defguard but couldn't propagate that change or did we delete the
//!   user from LDAP?
//! - If the authority is LDAP, we should delete the user from Defguard, as we assume that it was
//!   more probable that the change was made in LDAP.
//! - If the authority is Defguard, we should add the user to LDAP, as we assume that it was more
//!   probable that the change was made in Defguard.
//!
//! If the LDAP connection is never lost and no other issues arise, the full sync should be
//! performed only once, when the LDAP sync is enabled. So this is a more of a damage control
//! mechanism rather than something that should be invoked regularly.
//!
//! # Incremental synchronization
//!
//! The incremental synchronization is a regular synchronization operation which comes in two
//! varieties: synchronous and asynchronous.
//!
//! Changes from Defguard are propagated to LDAP in real-time, synchronously, to keep LDAP
//! up-to-date with Defguard instantly. This is done by calling appropriate LDAP operations after
//! each change in Defguard. Changes other way around (from LDAP to Defguard) are pulled
//! asynchronously at regular intervals (every 5 minutes by default). Implementation-wise it's done
//! by running a full sync with LDAP authority, as it has the same effect when we consider that LDAP
//! has the most recent Defguard changes (due to synchronous change propagation).
//!
//! This synchronization should work reliably most of the time, given that:
//! - LDAP connection is stable
//! - The LDAP change pull is performed relatively often
//! - One object is not changed in both sources between two asynchronous syncs (may cause
//!   overwriting of changes), but this sounds like an unlikely scenario
//!
//! # Potential improvements and issues
//!
//! - Some optimizations could be made using the implementation-specific object
//!   modification/creation timestamps in LDAP. Currently everything is compared as is, without any
//!   regard to the time of the change. We could skip some operations on objects that haven't
//!   changed since the last sync. There is however still an issue with objects that have been
//!   deleted, LDAP doesn't store deleted objects by default, so we may still need to compare full
//!   object lists.
//! - There is no real pagination and everything is loaded into the memory at once. This may be an
//!   issue at some point. 10k LDAP records wasn't a problem in testing. We may have bigger issues
//!   with other parts of Defguard with that user count though.
//!
use std::collections::{HashMap, HashSet};

use defguard_common::db::{
    Id,
    models::{
        Settings, User,
        group::Group,
        settings::{LdapSyncStatus, update_current_settings},
    },
};
use serde::Serialize;
use sqlx::{PgConnection, PgPool};
use tokio::sync::{broadcast::Sender, mpsc::UnboundedSender};

use super::{LDAPConfig, error::LdapError};
use crate::{
    enrollment_management::try_send_ldap_enrollment_invite,
    enterprise::{
        ldap::model::{
            get_users_without_ldap_path, ldap_sync_allowed_for_user,
            ldap_sync_allowed_for_user_scoped, update_from_ldap_user, user_from_searchentry,
        },
        license::get_cached_license,
        limits::{get_counts, update_counts},
    },
    events::LdapSyncEventType,
    grpc::GatewayCommand,
    hashset,
    user_management::{disable_user, sync_allowed_user_devices},
};

fn emit_ldap_sync_events(
    ldap_tx: &UnboundedSender<LdapSyncEventType>,
    events: Vec<LdapSyncEventType>,
) {
    for event in events {
        if let Err(err) = ldap_tx.send(event) {
            error!("Failed to send LDAP sync activity log event: {err}");
        }
    }
}

fn emit_ldap_sync_event(ldap_tx: &UnboundedSender<LdapSyncEventType>, event: LdapSyncEventType) {
    if let Err(err) = ldap_tx.send(event) {
        error!("Failed to send LDAP sync activity log event: {err}");
    }
}

async fn get_or_create_group(
    transaction: &mut PgConnection,
    groupname: &str,
) -> Result<(Group<Id>, bool), LdapError> {
    let group = if let Some(group) = Group::find_by_name(&mut *transaction, groupname).await? {
        debug!("Group {groupname} already exists, skipping creation");
        (group, false)
    } else {
        debug!("Group {groupname} didn't exist, creating it now");
        let new_group = Group::new(groupname).save(&mut *transaction).await?;
        debug!("Group {groupname} created");
        (new_group, true)
    };

    Ok(group)
}

#[derive(Debug, Clone, Copy)]
pub enum Authority {
    LDAP,
    Defguard,
}

#[must_use]
pub fn get_ldap_sync_status() -> LdapSyncStatus {
    let settings = Settings::get_current_settings();
    settings.ldap_sync_status
}

pub async fn set_ldap_sync_status(status: LdapSyncStatus, pool: &PgPool) -> Result<(), LdapError> {
    debug!("Setting LDAP sync status to {status:?}");
    let mut settings = Settings::get_current_settings();
    settings.ldap_sync_status = status;
    update_current_settings(pool, settings).await?;
    debug!("LDAP sync status set to {status:?}");
    Ok(())
}

#[must_use]
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

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LdapDryRunAction {
    Add,
    Remove,
}

#[derive(Debug, Serialize)]
pub struct LdapDryRunUser {
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub action: LdapDryRunAction,
}

/// Preview of the user changes a full sync would make, split by the system that would be
/// modified. Built from [`UserSyncChanges`].
#[derive(Debug, Serialize)]
pub struct LdapDryRunResult {
    pub defguard: Vec<LdapDryRunUser>,
    pub ldap: Vec<LdapDryRunUser>,
}

fn dry_run_user<I>(user: &User<I>, action: LdapDryRunAction) -> LdapDryRunUser {
    LdapDryRunUser {
        username: user.username.clone(),
        email: user.email.clone(),
        first_name: user.first_name.clone(),
        last_name: user.last_name.clone(),
        action,
    }
}

impl From<UserSyncChanges> for LdapDryRunResult {
    fn from(changes: UserSyncChanges) -> Self {
        let mut defguard = Vec::new();
        defguard.extend(
            changes
                .add_defguard
                .iter()
                .map(|u| dry_run_user(u, LdapDryRunAction::Add)),
        );
        defguard.extend(
            changes
                .delete_defguard
                .iter()
                .map(|u| dry_run_user(u, LdapDryRunAction::Remove)),
        );

        let mut ldap = Vec::new();
        ldap.extend(
            changes
                .add_ldap
                .iter()
                .map(|u| dry_run_user(u, LdapDryRunAction::Add)),
        );
        ldap.extend(
            changes
                .delete_ldap
                .iter()
                .map(|u| dry_run_user(u, LdapDryRunAction::Remove)),
        );

        Self { defguard, ldap }
    }
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
        .map(|u| ldap_config.user_dn_for_user(u))
        .collect::<HashSet<_>>();

    trace!("Defguard identifiers: {defguard_identifiers:?}");
    trace!("LDAP identifiers: {ldap_identifiers:?}");

    for user in all_ldap_users.drain(..) {
        ldap_identifiers.insert(ldap_config.user_dn_for_user(&user));

        debug!("Checking if user {} is in Defguard", user.username);
        if !defguard_identifiers.contains(&ldap_config.user_dn_for_user(&user)) {
            debug!("User {} not found in Defguard", user.username);
            match authority {
                Authority::LDAP => add_defguard.push(user),
                Authority::Defguard => delete_ldap.push(user),
            }
        }
    }

    for user in all_defguard_users.drain(..) {
        debug!("Checking if user {} is in LDAP", user.username);
        if !ldap_identifiers.contains(&ldap_config.user_dn_for_user(&user)) {
            debug!("User {} not found in LDAP", user.username);
            match authority {
                Authority::LDAP => {
                    // Skip inactive/not enrolled users when deleting from LDAP
                    if user.is_active && user.is_enrolled_or_ldap_pending() {
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
                    if user.is_active && user.is_enrolled_or_ldap_pending() {
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
    trace!("User sync changes: {user_sync_changes:?}");

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
    defguard_memberships: &HashMap<String, HashSet<User<Id>>>,
    ldap_memberships: HashMap<String, HashSet<&'a User>>,
    authority: Authority,
    ldap_config: &LDAPConfig,
) -> GroupSyncChanges<'a> {
    debug!("Computing group sync changes (group membership changes), authority: {authority:?}");
    let mut delete_defguard = HashMap::new();
    let mut add_defguard = HashMap::new();
    let mut delete_ldap = HashMap::new();
    let mut add_ldap = HashMap::new();

    for (group, members) in defguard_memberships {
        debug!("Checking group {} for changes", group);
        if let Some(ldap_members) = ldap_memberships.get(group) {
            debug!("Group {group:?} found in LDAP, checking for membership differences");
            let missing_from_defguard = ldap_members
                .iter()
                .filter(|u| {
                    !members
                        .iter()
                        .any(|m| ldap_config.user_dn_for_user(m) == ldap_config.user_dn_for_user(u))
                })
                .copied()
                .collect::<HashSet<_>>();

            let missing_from_ldap = members
                .iter()
                .filter(|m| {
                    !ldap_members
                        .iter()
                        .any(|u| ldap_config.user_dn_for_user(m) == ldap_config.user_dn_for_user(u))
                })
                .cloned()
                .collect::<HashSet<_>>();

            trace!(
                "Group {group:?} members missing from Defguard: {missing_from_defguard:?}, missing \
                from LDAP: {missing_from_ldap:?}"
            );

            if missing_from_defguard.is_empty() {
                debug!("Group {group:?} has no members missing from Defguard");
            } else {
                match authority {
                    Authority::Defguard => {
                        debug!(
                            "Group {group:?} has members missing from Defguard, marking them for \
                            deletion in LDAP: {missing_from_defguard:?}"
                        );
                        delete_ldap.insert(group.clone(), missing_from_defguard);
                    }
                    Authority::LDAP => {
                        debug!(
                            "Group {group:?} has members missing from Defguard, marking them for \
                            addition in Defguard: {missing_from_defguard:?}"
                        );
                        add_defguard.insert(group.clone(), missing_from_defguard);
                    }
                }
            }

            if missing_from_ldap.is_empty() {
                debug!("Group {group:?} has no members missing from LDAP");
            } else {
                match authority {
                    Authority::Defguard => {
                        debug!(
                            "Group {group:?} has members missing from LDAP, marking them for \
                            addition to LDAP: {missing_from_ldap:?}"
                        );
                        add_ldap.insert(group.clone(), missing_from_ldap);
                    }
                    Authority::LDAP => {
                        debug!(
                            "Group {group:?} has members missing from LDAP, marking them for \
                            deletion in Defguard: {missing_from_ldap:?}"
                        );
                        delete_defguard.insert(group.clone(), missing_from_ldap);
                    }
                }
            }
        } else {
            match authority {
                Authority::Defguard => {
                    debug!(
                        "Group {group:?} is missing from LDAP, marking it for addition to LDAP \
                        along with all members due to Defguard authority"
                    );
                    add_ldap.insert(group.clone(), members.clone());
                }
                Authority::LDAP => {
                    debug!(
                        "Group {group:?} is missing from LDAP, marking all its member for deletion \
                        from Defguard due to LDAP authority"
                    );
                    delete_defguard.insert(group.clone(), members.clone());
                }
            }
        }
    }

    for (group, members) in ldap_memberships {
        if !defguard_memberships.contains_key(&group) {
            match authority {
                Authority::Defguard => {
                    debug!(
                        "Group {group:?} is missing from Defguard, marking all its member for \
                        deletion from LDAP due to Defguard authority"
                    );
                    delete_ldap.insert(group, members);
                }
                Authority::LDAP => {
                    debug!(
                        "Group {group:?} is missing from Defguard, marking all its member for \
                        addition to Defguard due to LDAP authority"
                    );
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

/// Extracts users that are in both sources for later comparison and attritubte modification
/// (emails, phone numbers).
pub(super) fn extract_intersecting_users(
    defguard_users: &mut Vec<User<Id>>,
    ldap_users: &mut Vec<User>,
    ldap_config: &LDAPConfig,
) -> Vec<(User, User<Id>)> {
    let mut intersecting_users = Vec::new();
    let mut intersecting_users_ldap = Vec::new();

    for defguard_user in defguard_users.iter() {
        if let Some(ldap_user) = ldap_users
            .iter()
            .position(|u| {
                ldap_config.user_dn_for_user(u) == ldap_config.user_dn_for_user(defguard_user)
            })
            .map(|i| ldap_users.remove(i))
        {
            intersecting_users_ldap.push(ldap_user);
        }
    }

    for user in intersecting_users_ldap {
        if let Some(defguard_user) = defguard_users
            .iter()
            .position(|u| ldap_config.user_dn_for_user(u) == ldap_config.user_dn_for_user(&user))
            .map(|i| defguard_users.remove(i))
        {
            intersecting_users.push((user, defguard_user));
        }
    }

    intersecting_users
}

const DEFAULT_LDAP_SYNC_INTERVAL: u64 = 60 * 5;

#[must_use]
pub fn get_ldap_sync_interval() -> u64 {
    let settings = Settings::get_current_settings();
    settings
        .ldap_sync_interval
        .try_into()
        .unwrap_or(DEFAULT_LDAP_SYNC_INTERVAL)
}

impl super::LDAPConnection {
    /// Applies user modifications to users that are present in both LDAP and Defguard.
    async fn apply_user_modifications(
        &mut self,
        mut intersecting_users: Vec<(User, User<Id>)>,
        authority: Authority,
        pool: &PgPool,
        wg_tx: &Sender<GatewayCommand>,
        ldap_tx: &UnboundedSender<LdapSyncEventType>,
    ) -> Result<(), LdapError> {
        let sync_account_status = self.config.ldap_uses_ad && self.config.ldap_sync_account_status;
        let mut transaction = pool.begin().await?;
        let mut events = Vec::new();

        for (ldap_user, defguard_user) in &mut intersecting_users {
            if sync_account_status && ldap_user.is_active != defguard_user.is_active {
                match authority {
                    Authority::LDAP => {
                        if ldap_user.is_active {
                            debug!("Enabling Defguard user {defguard_user} based on AD status");
                            defguard_user.is_active = true;
                            defguard_user.save(&mut *transaction).await?;
                            sync_allowed_user_devices(defguard_user, &mut transaction, wg_tx)
                                .await
                                .map_err(|err| LdapError::UserStatusUpdate(err.to_string()))?;
                            events.push(LdapSyncEventType::UserEnabled {
                                user: defguard_user.clone(),
                            });
                        } else {
                            debug!("Disabling Defguard user {defguard_user} based on AD status");
                            disable_user(defguard_user, &mut transaction, wg_tx)
                                .await
                                .map_err(|err| LdapError::UserStatusUpdate(err.to_string()))?;
                            events.push(LdapSyncEventType::UserDisabled {
                                user: defguard_user.clone(),
                            });
                        }
                    }
                    Authority::Defguard => {
                        debug!("Applying Defguard account status to AD for {defguard_user}");
                        self.set_ad_account_status(defguard_user, defguard_user.is_active)
                            .await?;
                        let event = if defguard_user.is_active {
                            LdapSyncEventType::OutboundUserEnabled {
                                user: defguard_user.clone(),
                            }
                        } else {
                            LdapSyncEventType::OutboundUserDisabled {
                                user: defguard_user.clone(),
                            }
                        };
                        events.push(event);
                    }
                }
            }

            if attrs_different(defguard_user, ldap_user, &self.config) {
                debug!(
                    "User {defguard_user} attributes differ between LDAP and Defguard, merging..."
                );
                match authority {
                    Authority::LDAP => {
                        debug!("Applying LDAP user attributes to Defguard user");
                        let before = defguard_user.clone();
                        update_from_ldap_user(defguard_user, ldap_user, &self.config);
                        defguard_user.save(&mut *transaction).await?;
                        events.push(LdapSyncEventType::UserModified {
                            before,
                            after: defguard_user.clone(),
                        });
                    }
                    Authority::Defguard => {
                        debug!("Applying Defguard user attributes to LDAP user");
                        self.modify_user(&ldap_user.username, defguard_user).await?;
                        events.push(LdapSyncEventType::OutboundUserModified {
                            user: defguard_user.clone(),
                        });
                    }
                }
            }
        }

        transaction.commit().await?;
        emit_ldap_sync_events(ldap_tx, events);

        Ok(())
    }

    /// Allows to synchronize user data (e.g. email, groups) between Defguard and LDAP based on the
    /// authority for a single user.
    ///
    /// Does nothing if the two way sync is disabled.
    pub(crate) async fn sync_user_data(
        &mut self,
        user: &User<Id>,
        pool: &PgPool,
        wg_tx: &Sender<GatewayCommand>,
        ldap_tx: &UnboundedSender<LdapSyncEventType>,
    ) -> Result<(), LdapError> {
        debug!("Syncing user data for {user}");
        let settings = Settings::get_current_settings();

        // Force update user data in LDAP if the two-way sync is disabled, otherwise respect the
        // authority.
        let authority = if !settings.ldap_sync_enabled || !settings.ldap_is_authoritative {
            Authority::Defguard
        } else {
            Authority::LDAP
        };

        let user_dn = self.config.user_dn_for_user(user);
        let ldap_user = self.get_user_by_dn(user).await?;
        let defguard_groups = user.member_of_names(pool).await?;
        let ldap_groups = self.get_user_groups(&user_dn).await?;

        debug!("User {user} is a member of the following groups in Defguard: {defguard_groups:?}");
        debug!("User {user} is a member of the following groups in LDAP: {ldap_groups:?}");

        let intersecting_users = vec![(ldap_user.clone(), user.clone())];

        // Create a hashmap for the calculated group membership changes function.
        let defguard_memberships = defguard_groups
            .iter()
            .map(|g| (g.clone(), hashset![user.clone()]))
            .collect::<HashMap<_, _>>();

        let ldap_memberships = ldap_groups
            .iter()
            .map(|g| (g.clone(), hashset![&ldap_user]))
            .collect::<HashMap<_, _>>();

        self.apply_user_modifications(intersecting_users, authority, pool, wg_tx, ldap_tx)
            .await?;

        let changes = compute_group_sync_changes(
            &defguard_memberships,
            ldap_memberships,
            authority,
            &self.config,
        );
        self.apply_user_group_sync_changes(pool, changes, ldap_tx)
            .await?;

        Ok(())
    }

    /// Fixes users with missing LDAP path
    /// This is for compatibility with older Defguard versions that didn't store LDAP paths in the
    /// database.
    /// It will try to fetch the LDAP path from the LDAP server for users that have it missing.
    /// If the user is not found in LDAP, it will skip fixing that user.
    ///
    /// This function matches the user by username first, as those should be unique in both Defguard
    /// and LDAP. Then, just to make sure the user wasn't renamed, it checks if the RDN values
    /// match.
    pub(crate) async fn fix_missing_user_path(&mut self, pool: &PgPool) -> Result<(), LdapError> {
        debug!("Fixing missing user path in LDAP");

        let mut transaction = pool.begin().await?;
        let users = get_users_without_ldap_path(&mut *transaction).await?;

        let mut filtered_users = Vec::new();
        for user in users {
            if ldap_sync_allowed_for_user(&user, &mut *transaction).await? {
                filtered_users.push(user);
            }
        }
        let users = filtered_users;

        for mut defguard_user in users {
            if defguard_user.ldap_user_path.is_none() {
                match self.get_user_by_username(&defguard_user.username).await {
                    Ok(ldap_user) => {
                        debug!(
                            "Found LDAP user {} with missing path in Defguard, fixing their path",
                            defguard_user.username
                        );
                        let defguard_user_rdn = defguard_user.ldap_rdn_value();
                        let ldap_user_rdn = ldap_user.ldap_rdn_value();

                        if defguard_user_rdn != ldap_user_rdn {
                            warn!(
                                "User {} has different RDN in Defguard ({defguard_user_rdn}) and \
                                LDAP ({ldap_user_rdn}), cannot fix missing LDAP path. Please, \
                                update the username manually, so it matches in both sources.",
                                defguard_user.username,
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
                                "User {} has no LDAP path in LDAP, skipping fixing their path in \
                                Defguard",
                                defguard_user.username
                            );
                        }
                    }
                    Err(err) => {
                        debug!(
                            "Failed to get user {} from LDAP: {err}, cannot update their DN in \
                            Defguard",
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
    pub async fn sync(
        &mut self,
        pool: &PgPool,
        full: bool,
        wg_tx: &Sender<GatewayCommand>,
        ldap_tx: &UnboundedSender<LdapSyncEventType>,
    ) -> Result<(), LdapError> {
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

        debug!(
            "The following groups were defined for sync: {sync_groups:?}, only Defguard users \
            belonging to these groups will be synced"
        );
        let mut sync_group_members = HashSet::new();
        for sync_group in &sync_groups {
            let members = sync_group.members(pool).await?;
            sync_group_members.extend(members);
        }

        let (mut all_ldap_users, mut all_defguard_users) = self.get_sync_users(pool).await?;

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
                if ldap_sync_allowed_for_user(&member, pool).await? {
                    members.insert(member);
                }
            }
            defguard_memberships.insert(group.name, members);
        }

        let intersecting_users =
            extract_intersecting_users(&mut all_defguard_users, &mut all_ldap_users, &self.config);

        self.apply_user_modifications(intersecting_users, authority, pool, wg_tx, ldap_tx)
            .await?;

        let user_changes = compute_user_sync_changes(
            &mut all_ldap_users,
            &mut all_defguard_users,
            authority,
            &self.config,
        );

        let membership_changes = compute_group_sync_changes(
            &defguard_memberships,
            ldap_memberships,
            authority,
            &self.config,
        );

        self.apply_user_sync_changes(pool, user_changes, ldap_tx)
            .await?;
        self.apply_user_group_sync_changes(pool, membership_changes, ldap_tx)
            .await?;

        if full {
            debug!("Full LDAP sync completed");
        } else {
            debug!("LDAP Incremental sync completed");
        }

        Ok(())
    }

    /// Fetches all LDAP users alongside the Defguard users that are allowed to participate in
    /// sync, filtering out the ones that should be ignored.
    async fn get_sync_users(
        &mut self,
        pool: &PgPool,
    ) -> Result<(Vec<User>, Vec<User<Id>>), LdapError> {
        let all_ldap_users = self.get_all_users().await?;
        let all_defguard_users = User::all(pool).await?;

        let sync_account_status = self.config.ldap_uses_ad && self.config.ldap_sync_account_status;
        let mut filtered_users = Vec::new();
        for user in all_defguard_users {
            if ldap_sync_allowed_for_user_scoped(
                &user,
                pool,
                sync_account_status,
                &self.config.ldap_sync_groups,
            )
            .await?
            {
                filtered_users.push(user);
            }
        }

        Ok((all_ldap_users, filtered_users))
    }

    /// Computes the user additions/removals a full sync would perform, without applying any
    /// of them.
    pub async fn dry_run(
        &mut self,
        pool: &PgPool,
        authority: Authority,
    ) -> Result<LdapDryRunResult, LdapError> {
        debug!("Performing LDAP dry run with authority: {authority:?}");

        let (mut all_ldap_users, mut all_defguard_users) = self.get_sync_users(pool).await?;

        // Mirror `fix_missing_user_path()` in memory.
        let ldap_paths_by_username: HashMap<&str, (&str, Option<&str>)> = all_ldap_users
            .iter()
            .map(|u| {
                (
                    u.username.as_str(),
                    (u.ldap_rdn_value(), u.ldap_user_path.as_deref()),
                )
            })
            .collect();
        for defguard_user in &mut all_defguard_users {
            if defguard_user.ldap_user_path.is_some() {
                continue;
            }
            if let Some((ldap_rdn, ldap_path)) =
                ldap_paths_by_username.get(defguard_user.username.as_str())
                && defguard_user.ldap_rdn_value() == *ldap_rdn
            {
                defguard_user.ldap_user_path = ldap_path.map(str::to_owned);
            }
        }

        let mut user_changes = compute_user_sync_changes(
            &mut all_ldap_users,
            &mut all_defguard_users,
            authority,
            &self.config,
        );

        let existing_usernames = User::all(pool)
            .await?
            .into_iter()
            .map(|user| user.username)
            .collect::<HashSet<_>>();
        user_changes
            .add_defguard
            .retain(|user| !existing_usernames.contains(&user.username));

        debug!("LDAP dry run completed");
        Ok(LdapDryRunResult::from(user_changes))
    }

    async fn apply_user_group_sync_changes(
        &mut self,
        pool: &PgPool,
        changes: GroupSyncChanges<'_>,
        ldap_tx: &UnboundedSender<LdapSyncEventType>,
    ) -> Result<(), LdapError> {
        debug!("Applying group memberships sync changes");
        let mut transaction = pool.begin().await?;
        let mut admin_count = User::find_admins(&mut *transaction).await?.len();
        let mut events = Vec::new();
        for (groupname, members) in changes.delete_defguard {
            if members.is_empty() {
                debug!("No members to remove from group {groupname}, skipping");
                continue;
            }
            let (group, group_created) = get_or_create_group(&mut transaction, &groupname).await?;
            if group_created {
                events.push(LdapSyncEventType::GroupCreated {
                    group: group.clone(),
                });
            }

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
                        events.push(LdapSyncEventType::GroupMemberRemoved {
                            group: group.clone(),
                            user: member,
                        });
                    }
                } else {
                    debug!("Removing user {} from group {}", member.username, groupname);
                    member.remove_from_group(&mut *transaction, &group).await?;
                    events.push(LdapSyncEventType::GroupMemberRemoved {
                        group: group.clone(),
                        user: member,
                    });
                }
            }
        }

        for (groupname, members) in changes.add_defguard {
            if members.is_empty() {
                debug!("No members to add to group {groupname}, skipping");
                continue;
            }
            let (group, group_created) = get_or_create_group(&mut transaction, &groupname).await?;
            if group_created {
                events.push(LdapSyncEventType::GroupCreated {
                    group: group.clone(),
                });
            }
            for member in members {
                if let Some(user) =
                    User::find_by_username(&mut *transaction, &member.username).await?
                {
                    user.add_to_group(&mut *transaction, &group).await?;
                    events.push(LdapSyncEventType::GroupMemberAdded {
                        group: group.clone(),
                        user,
                    });
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
        emit_ldap_sync_events(ldap_tx, events);

        for (groupname, members) in changes.delete_ldap {
            for member in members {
                self.remove_user_from_group(member, &groupname).await?;
                emit_ldap_sync_event(
                    ldap_tx,
                    LdapSyncEventType::OutboundGroupMemberRemoved {
                        group: groupname.clone(),
                        username: member.username.clone(),
                    },
                );
            }
        }

        for (groupname, members) in changes.add_ldap {
            for member in members {
                self.add_user_to_group(&member, &groupname).await?;
                emit_ldap_sync_event(
                    ldap_tx,
                    LdapSyncEventType::OutboundGroupMemberAdded {
                        group: groupname.clone(),
                        username: member.username,
                    },
                );
            }
        }

        Ok(())
    }

    async fn apply_user_sync_changes(
        &mut self,
        pool: &PgPool,
        mut changes: UserSyncChanges,
        ldap_tx: &UnboundedSender<LdapSyncEventType>,
    ) -> Result<(), LdapError> {
        let mut transaction = pool.begin().await?;
        let mut admin_count = User::find_admins(&mut *transaction).await?.len();
        let mut user_count = get_counts().user();
        let mut events = Vec::new();

        let user_limit = get_cached_license()
            .as_ref()
            .and_then(|license| license.limits.as_ref().map(|limits| limits.users));
        let mut blocked_import_notification_sent = false;

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
                    let deleted_user = user.clone();
                    user.delete(&mut *transaction).await?;
                    events.push(LdapSyncEventType::UserDeleted { user: deleted_user });
                }
            } else {
                debug!("Deleting user {} from Defguard", user.username);
                let deleted_user = user.clone();
                user.delete(&mut *transaction).await?;
                events.push(LdapSyncEventType::UserDeleted { user: deleted_user });
            }
        }

        let mut new_users = Vec::new();
        for user in changes.add_defguard {
            debug!("Adding user {} to Defguard", user.username);
            if let Some(defguard_user) =
                User::find_by_username(&mut *transaction, &user.username).await?
            {
                let defguard_user_dn = self.config.user_dn_for_user(&defguard_user);
                let ldap_user_dn = self.config.user_dn_for_user(&user);
                if defguard_user_dn == ldap_user_dn {
                    debug!(
                        "User {} (DN: {}) already exists in Defguard, skipping...",
                        user.username, defguard_user_dn
                    );
                } else {
                    warn!(
                        "LDAP user with username {} already exists in Defguard. Those users have \
                        different DNs: {ldap_user_dn} (Defguard) vs {defguard_user_dn} (LDAP). All \
                        usernames must be unique, so this LDAP user will not be added to Defguard.",
                        user.username,
                    );
                }
            } else {
                debug!(
                    "LDAP user {} does not exist in Defguard yet, adding...",
                    user.username
                );
                if let Some(limit) = user_limit.filter(|limit| user_count >= *limit) {
                    error!(
                        "Skipping LDAP import of user {} (email: {}) because license user limit \
                        has been reached ({user_count}/{limit})",
                        user.username, user.email
                    );
                    if !blocked_import_notification_sent {
                        blocked_import_notification_sent = true;
                        // TODO: send emails
                    }
                    continue;
                }
                let saved_user = user.save(&mut *transaction).await?;
                events.push(LdapSyncEventType::UserCreated {
                    user: saved_user.clone(),
                });
                new_users.push(saved_user);
                user_count += 1;
            }
        }

        transaction.commit().await?;
        emit_ldap_sync_events(ldap_tx, events);

        // attempt to send enrollment invites after the original DB transaction is commited
        // and users actually exist in DB
        let mut transaction = pool.begin().await?;
        for mut user in new_users {
            try_send_ldap_enrollment_invite(&mut user, &mut transaction).await;
        }
        transaction.commit().await?;

        update_counts(pool).await?;

        for user in changes.delete_ldap {
            debug!("Deleting user {} from LDAP", user.username);
            self.delete_user(&user).await?;
            emit_ldap_sync_event(
                ldap_tx,
                LdapSyncEventType::OutboundUserDeleted {
                    username: user.username.clone(),
                },
            );
        }

        for user in &mut changes.add_ldap {
            debug!("Adding user {} to LDAP", user.username);
            self.add_user(user, None, pool).await?;
            emit_ldap_sync_event(
                ldap_tx,
                LdapSyncEventType::OutboundUserCreated { user: user.clone() },
            );
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

            match user_from_searchentry(&entry, username, None, &self.config) {
                Ok(user) => all_users.push(user),
                Err(err) => {
                    warn!(
                        "Failed to create user {username} from LDAP entry, error: {err}. The user \
                        will be skipped during sync"
                    );
                    debug!("Skipping user {username} due to error: {err}");
                }
            }
        }

        Ok(all_users)
    }
}
