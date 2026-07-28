use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool, query_scalar};
use utoipa::ToSchema;

use crate::{
    db::{
        Id,
        models::{MFAMethod, Settings, device::UserDevice, group::Group, user::User},
    },
    types::group_diff::GroupDiff,
};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OAuth2AuthorizedAppInfo {
    pub oauth2client_id: Id,
    pub oauth2client_name: String,
}

// Basic user info used in user list, etc.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UserInfo {
    pub id: Id,
    pub username: String,
    pub last_name: String,
    pub first_name: String,
    name: String,
    pub email: String,
    pub phone: Option<String>,
    pub mfa_enabled: bool,
    pub totp_enabled: bool,
    pub email_mfa_enabled: bool,
    pub groups: Vec<String>,
    pub mfa_method: MFAMethod,
    pub authorized_apps: Vec<OAuth2AuthorizedAppInfo>,
    pub is_active: bool,
    pub enrolled: bool,
    pub is_admin: bool,
    pub ldap_pass_requires_change: bool,
    pub password_management_disabled: bool,
    pub devices: Vec<UserDevice>,
    pub has_non_mfa_location_access: bool,
    pub has_non_posture_location_access: bool,
}

/// Check whether any network with MFA disabled is accessible to a user
/// based on their group names.
async fn has_non_mfa_location_access(pool: &PgPool, groups: &[String]) -> sqlx::Result<bool> {
    query_scalar!(
        "SELECT EXISTS( \
            SELECT 1 FROM wireguard_network wn \
            WHERE wn.location_mfa_mode = 'disabled' \
            AND ( \
                wn.allow_all_groups \
                OR EXISTS( \
                    SELECT 1 FROM wireguard_network_allowed_group wnag \
                    JOIN \"group\" g ON g.id = wnag.group_id \
                    WHERE wnag.network_id = wn.id \
                    AND g.name = ANY($1) \
                ) \
            ) \
        )",
        groups,
    )
    .fetch_one(pool)
    .await
    .map(|v| v.unwrap_or(false))
}

/// Check whether any network without posture checks assigned is accessible to a
/// user based on their group names.
async fn has_non_posture_location_access(pool: &PgPool, groups: &[String]) -> sqlx::Result<bool> {
    query_scalar!(
        "SELECT EXISTS( \
            SELECT 1 FROM wireguard_network wn \
            WHERE NOT EXISTS( \
                SELECT 1 FROM device_posture_location dpl WHERE dpl.location_id = wn.id \
            ) \
            AND ( \
                wn.allow_all_groups \
                OR EXISTS( \
                    SELECT 1 FROM wireguard_network_allowed_group wnag \
                    JOIN \"group\" g ON g.id = wnag.group_id \
                    WHERE wnag.network_id = wn.id \
                    AND g.name = ANY($1) \
                ) \
            ) \
        )",
        groups,
    )
    .fetch_one(pool)
    .await
    .map(|v| v.unwrap_or(false))
}

impl UserInfo {
    /// Convert [`User`] to [`UserInfo`].
    pub async fn from_user(
        pool: &PgPool,
        user: User<Id>,
        // FIXME: remove this and just fetch straight from DB once we reorganize enterprise code to allow required imports here
        oidc_disable_password_management: bool,
    ) -> sqlx::Result<Self> {
        let name = format!("{} {}", user.first_name, user.last_name);
        let groups = user.member_of_names(pool).await?;
        let authorized_apps = user.oauth2authorizedapps(pool).await?;
        let enrolled = user.is_enrolled();
        let is_admin = user.is_admin(pool).await?;
        let devices = user.user_devices(pool).await?;
        let settings = Settings::get_current_settings();
        let password_management_disabled = user.password_management_disabled(
            is_admin,
            &settings,
            oidc_disable_password_management,
        );

        let has_non_mfa_location_access = has_non_mfa_location_access(pool, &groups).await?;
        let has_non_posture_location_access =
            has_non_posture_location_access(pool, &groups).await?;

        Ok(Self {
            id: user.id,
            username: user.username,
            last_name: user.last_name,
            first_name: user.first_name,
            name,
            email: user.email,
            phone: user.phone,
            mfa_enabled: user.mfa_enabled,
            totp_enabled: user.totp_enabled,
            email_mfa_enabled: user.email_mfa_enabled,
            groups,
            mfa_method: user.mfa_method,
            authorized_apps,
            is_active: user.is_active,
            enrolled,
            is_admin,
            ldap_pass_requires_change: user.ldap_pass_randomized,
            password_management_disabled,
            devices,
            has_non_mfa_location_access,
            has_non_posture_location_access,
        })
    }

    /// Copy status to [`User`]. This function should be used by administrators.
    ///
    /// Return `true` if status was changed, `false` otherwise.
    /// If status was changed to inactive, all user sessions will be invalidated.
    pub async fn handle_status_change(
        &self,
        transaction: &mut PgConnection,
        user: &mut User<Id>,
    ) -> sqlx::Result<bool> {
        if self.is_active == user.is_active {
            Ok(false)
        } else {
            if !self.is_active {
                user.logout_all_sessions(&mut *transaction).await?;
            }
            user.is_active = self.is_active;
            user.save(&mut *transaction).await?;
            Ok(true)
        }
    }

    /// Copy groups to [`User`]. This function should be used by administrators.
    ///
    /// Return `true` if groups were changed, `false` otherwise.
    pub async fn handle_user_groups(
        &self,
        transaction: &mut PgConnection,
        user: &mut User<Id>,
    ) -> sqlx::Result<GroupDiff> {
        // initialize return value
        let mut group_diff = GroupDiff::default();

        // handle groups
        let mut present_groups = user.member_of(&mut *transaction).await?;

        // add to groups if not already a member
        for groupname in &self.groups {
            match present_groups
                .iter()
                .position(|group| &group.name == groupname)
            {
                Some(index) => {
                    present_groups.swap_remove(index);
                }
                None => {
                    if let Some(group) = Group::find_by_name(&mut *transaction, groupname).await? {
                        user.add_to_group(&mut *transaction, &group).await?;
                        group_diff.added.insert(group.name);
                    }
                }
            }
        }

        // remove from remaining groups
        for group in present_groups {
            user.remove_from_group(&mut *transaction, &group).await?;
            group_diff.removed.insert(group.name);
        }

        Ok(group_diff)
    }

    /// Copy fields over to the given [`User`].
    /// Additional flags control which fields are copied over.
    pub fn handle_update_user_fields(
        self,
        user: &mut User<Id>,
        is_admin: bool,
        is_updating_self: bool,
    ) {
        if is_admin {
            user.username = self.username;
            user.last_name = self.last_name;
            user.first_name = self.first_name;
            user.email = self.email;
        }

        if is_updating_self {
            user.mfa_method = self.mfa_method;
        }

        user.phone = self.phone;
    }
}

#[cfg(test)]
mod test {
    use std::{slice::from_ref, str::FromStr};

    use ipnetwork::IpNetwork;
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

    use super::*;
    use crate::{
        config::{DefGuardConfig, SERVER_CONFIG},
        db::{
            models::{
                MFAMethod,
                group::Group,
                settings::initialize_current_settings,
                user::User,
                wireguard::{LocationMfaMode, ServiceLocationMode, WireguardNetwork},
            },
            setup_pool,
        },
    };

    /// Build a minimal `UserInfo` from an existing saved `User<Id>`.
    /// Only the fields exercised by `handle_update_user_fields` need to be set
    /// here; the rest are left at their DB-loaded defaults.
    async fn user_info_from_db(pool: &PgPool, username: &str) -> (UserInfo, User<Id>) {
        let user = User::find_by_username(pool, username)
            .await
            .unwrap()
            .unwrap();
        let info = UserInfo::from_user(pool, user.clone(), false)
            .await
            .unwrap();
        (info, user)
    }

    #[sqlx::test]
    async fn test_user_info(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        initialize_current_settings(&pool).await.unwrap();

        let user = User::new(
            "hpotter",
            Some("pass123"),
            "Potter",
            "Harry",
            "h.potter@hogwart.edu.uk",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let group1 = Group::new("Gryffindor").save(&pool).await.unwrap();
        let group2 = Group::new("Hufflepuff").save(&pool).await.unwrap();
        let group3 = Group::new("Ravenclaw").save(&pool).await.unwrap();
        let group4 = Group::new("Slytherin").save(&pool).await.unwrap();

        user.add_to_group(&pool, &group1).await.unwrap();
        user.add_to_group(&pool, &group2).await.unwrap();

        let mut user_info = UserInfo::from_user(&pool, user, false).await.unwrap();
        assert_eq!(user_info.groups, ["Gryffindor", "Hufflepuff"]);

        user_info.groups = vec!["Gryffindor".into(), "Ravenclaw".into()];
        let mut user = User::find_by_username(&pool, "hpotter")
            .await
            .unwrap()
            .unwrap();

        let mut transaction = pool.begin().await.unwrap();
        user_info
            .handle_user_groups(&mut transaction, &mut user)
            .await
            .unwrap();
        // admin updating their own account: is_admin=true, is_updating_self=true
        user_info.handle_update_user_fields(&mut user, true, true);
        transaction.commit().await.unwrap();

        assert_eq!(group1.member_usernames(&pool).await.unwrap(), ["hpotter"]);
        assert_eq!(group3.member_usernames(&pool).await.unwrap(), ["hpotter"]);
        assert!(group2.member_usernames(&pool).await.unwrap().is_empty());
        assert!(group4.member_usernames(&pool).await.unwrap().is_empty());
    }

    // Admin updating another user must be able to change all profile
    // fields (username, first/last name, email) and phone, but NOT mfa_method.
    #[sqlx::test]
    async fn test_handle_update_admin_updating_other_user(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        initialize_current_settings(&pool).await.unwrap();
        let mut user = User::new(
            "hpotter",
            Some("pass123"),
            "Potter",
            "Harry",
            "h.potter@hogwart.edu.uk",
            Some("+48100200300".to_owned()),
        )
        .save(&pool)
        .await
        .unwrap();

        let (mut info, _) = user_info_from_db(&pool, "hpotter").await;
        info.username = "h_potter_new".into();
        info.first_name = "UpdatedFirst".into();
        info.last_name = "Pot".into();
        info.email = "updated@hogwart.edu.uk".into();
        info.phone = Some("+48999888777".into());
        info.mfa_method = MFAMethod::OneTimePassword;

        // is_admin=true, is_updating_self=false (admin editing someone else)
        info.handle_update_user_fields(&mut user, true, false);

        assert_eq!(user.username, "h_potter_new");
        assert_eq!(user.first_name, "UpdatedFirst");
        assert_eq!(user.last_name, "Pot");
        assert_eq!(user.email, "updated@hogwart.edu.uk");
        assert_eq!(user.phone, Some("+48999888777".into()));
        // mfa_method must NOT change because is_updating_self=false
        assert_eq!(user.mfa_method, MFAMethod::None);
    }

    // A regular user updating themselves may only change phone and
    // mfa_method; name/email fields must be left untouched.
    #[sqlx::test]
    async fn test_handle_update_non_admin_updating_self(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        initialize_current_settings(&pool).await.unwrap();
        let mut user = User::new(
            "hpotter",
            Some("pass123"),
            "Potter",
            "Harry",
            "h.potter@hogwart.edu.uk",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let (mut info, _) = user_info_from_db(&pool, "hpotter").await;
        info.username = "changed_username".into();
        info.first_name = "UpdatedFirst".into();
        info.last_name = "UpdatedLast".into();
        info.email = "updated@example.com".into();
        info.phone = Some("+48111222333".into());
        info.mfa_method = MFAMethod::OneTimePassword;

        // is_admin=false, is_updating_self=true
        info.handle_update_user_fields(&mut user, false, true);

        // profile fields must remain unchanged
        assert_eq!(user.username, "hpotter");
        assert_eq!(user.first_name, "Harry");
        assert_eq!(user.last_name, "Potter");
        assert_eq!(user.email, "h.potter@hogwart.edu.uk");
        // phone and mfa_method are always allowed
        assert_eq!(user.phone, Some("+48111222333".into()));
        assert_eq!(user.mfa_method, MFAMethod::OneTimePassword);
    }

    // A non-admin modifying ANOTHER user must not be able to change
    // any protected field, and mfa_method must also stay unchanged because
    // is_updating_self=false.
    #[sqlx::test]
    async fn test_handle_update_non_admin_updating_other_user(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        initialize_current_settings(&pool).await.unwrap();
        let mut user = User::new(
            "hpotter",
            Some("pass123"),
            "Potter",
            "Harry",
            "h.potter@hogwart.edu.uk",
            Some("+48100200300".to_owned()),
        )
        .save(&pool)
        .await
        .unwrap();
        let original_mfa = user.mfa_method;

        let (mut info, _) = user_info_from_db(&pool, "hpotter").await;
        info.username = "changed_username".into();
        info.first_name = "UpdatedFirst".into();
        info.last_name = "UpdatedLast".into();
        info.email = "updated@example.com".into();
        info.phone = Some("+48000000000".into());
        info.mfa_method = MFAMethod::OneTimePassword;

        // is_admin=false, is_updating_self=false
        info.handle_update_user_fields(&mut user, false, false);

        // only phone changes; everything else stays the same
        assert_eq!(user.username, "hpotter");
        assert_eq!(user.first_name, "Harry");
        assert_eq!(user.last_name, "Potter");
        assert_eq!(user.email, "h.potter@hogwart.edu.uk");
        assert_eq!(user.phone, Some("+48000000000".into()));
        assert_eq!(user.mfa_method, original_mfa);
    }

    // Admin updating their own account can change all fields
    // including mfa_method.
    #[sqlx::test]
    async fn test_handle_update_admin_updating_self(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        initialize_current_settings(&pool).await.unwrap();
        let mut user = User::new(
            "admin",
            Some("pass123"),
            "Admin",
            "Super",
            "admin@defguard",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let (mut info, _) = user_info_from_db(&pool, "admin").await;
        info.username = "admin_renamed".into();
        info.first_name = "NewFirst".into();
        info.last_name = "NewLast".into();
        info.email = "new@defguard".into();
        info.phone = Some("+48777888999".into());
        info.mfa_method = MFAMethod::OneTimePassword;

        // is_admin=true, is_updating_self=true
        info.handle_update_user_fields(&mut user, true, true);

        assert_eq!(user.username, "admin_renamed");
        assert_eq!(user.first_name, "NewFirst");
        assert_eq!(user.last_name, "NewLast");
        assert_eq!(user.email, "new@defguard");
        assert_eq!(user.phone, Some("+48777888999".into()));
        assert_eq!(user.mfa_method, MFAMethod::OneTimePassword);
    }

    /// User with no groups and no networks: should be false.
    #[sqlx::test]
    async fn test_no_networks_returns_false(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let groups: Vec<String> = vec![];
        let result = has_non_mfa_location_access(&pool, &groups).await.unwrap();
        assert!(!result);
    }

    /// allow_all_groups network with MFA disabled: any user should have access.
    #[sqlx::test]
    async fn test_allow_all_groups_disabled_mfa(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        WireguardNetwork::new(
            "open-network".to_owned(),
            50051,
            String::new(),
            None,
            [IpNetwork::from_str("10.1.1.0/24").unwrap()],
            true, // allow_all_groups
            false,
            false,
            false,
            LocationMfaMode::Disabled,
            ServiceLocationMode::Disabled,
        )
        .set_address([IpNetwork::from_str("10.1.1.1/24").unwrap()])
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

        let groups: Vec<String> = vec![];
        let result = has_non_mfa_location_access(&pool, &groups).await.unwrap();
        assert!(result);
    }

    /// Network restricted to a specific group, user is a member, MFA disabled.
    #[sqlx::test]
    async fn test_group_member_has_access(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let group = Group::new("engineering").save(&pool).await.unwrap();

        let network = WireguardNetwork::new(
            "eng-network".to_owned(),
            50052,
            String::new(),
            None,
            [IpNetwork::from_str("10.2.1.0/24").unwrap()],
            false, // not allow_all_groups
            false,
            false,
            false,
            LocationMfaMode::Disabled,
            ServiceLocationMode::Disabled,
        )
        .set_address([IpNetwork::from_str("10.2.1.1/24").unwrap()])
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

        let mut transaction = pool.begin().await.unwrap();
        network
            .set_allowed_groups(&mut transaction, from_ref(&group.name))
            .await
            .unwrap();
        transaction.commit().await.unwrap();

        let groups = vec!["engineering".to_owned()];
        let result = has_non_mfa_location_access(&pool, &groups).await.unwrap();
        assert!(result);
    }

    /// Network restricted to a group the user is NOT in, MFA disabled.
    #[sqlx::test]
    async fn test_non_member_has_no_access(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let group = Group::new("engineering").save(&pool).await.unwrap();

        let network = WireguardNetwork::new(
            "eng-network".to_owned(),
            50053,
            String::new(),
            None,
            [IpNetwork::from_str("10.3.1.0/24").unwrap()],
            false,
            false,
            false,
            false, // not allow_all_groups
            LocationMfaMode::Disabled,
            ServiceLocationMode::Disabled,
        )
        .set_address([IpNetwork::from_str("10.3.1.1/24").unwrap()])
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

        let mut transaction = pool.begin().await.unwrap();
        network
            .set_allowed_groups(&mut transaction, &[group.name])
            .await
            .unwrap();
        transaction.commit().await.unwrap();

        let groups: Vec<String> = vec![];
        let result = has_non_mfa_location_access(&pool, &groups).await.unwrap();
        assert!(!result);
    }

    /// Network with allow_all_groups but MFA is internal: should still be false.
    #[sqlx::test]
    async fn test_mfa_internal_blocks_access(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        WireguardNetwork::new(
            "mfa-network".to_owned(),
            50054,
            String::new(),
            None,
            [IpNetwork::from_str("10.4.1.0/24").unwrap()],
            true, // allow_all_groups
            false,
            false,
            false,
            LocationMfaMode::Internal,
            ServiceLocationMode::Disabled,
        )
        .set_address([IpNetwork::from_str("10.4.1.1/24").unwrap()])
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

        let groups: Vec<String> = vec!["any-group".into()];
        let result = has_non_mfa_location_access(&pool, &groups).await.unwrap();
        assert!(!result);
    }

    /// Two networks: one MFA-enabled accessible, one MFA-disabled accessible → true.
    #[sqlx::test]
    async fn test_mixed_networks_when_one_disabled(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let group = Group::new("ops").save(&pool).await.unwrap();

        // MFA-disabled network for the ops group.
        let mfa_disabled_net = WireguardNetwork::new(
            "ops-net".to_owned(),
            50055,
            String::new(),
            None,
            [IpNetwork::from_str("10.5.1.0/24").unwrap()],
            false,
            false,
            false,
            false,
            LocationMfaMode::Disabled,
            ServiceLocationMode::Disabled,
        )
        .set_address([IpNetwork::from_str("10.5.1.1/24").unwrap()])
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

        // MFA-internal network that allow_all_groups (anyone, but MFA required).
        WireguardNetwork::new(
            "mfa-net".to_owned(),
            50056,
            String::new(),
            None,
            [IpNetwork::from_str("10.6.1.0/24").unwrap()],
            true, // allow_all_groups
            false,
            false,
            false,
            LocationMfaMode::Internal,
            ServiceLocationMode::Disabled,
        )
        .set_address([IpNetwork::from_str("10.6.1.1/24").unwrap()])
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

        let mut transaction = pool.begin().await.unwrap();
        mfa_disabled_net
            .set_allowed_groups(&mut transaction, from_ref(&group.name))
            .await
            .unwrap();
        transaction.commit().await.unwrap();

        let groups = vec!["ops".to_owned()];
        let result = has_non_mfa_location_access(&pool, &groups).await.unwrap();
        assert!(result);
    }

    #[sqlx::test]
    async fn test_no_posture_location_returns_true(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        WireguardNetwork::new(
            "no-posture-net".to_owned(),
            50057,
            String::new(),
            None,
            [IpNetwork::from_str("10.7.1.0/24").unwrap()],
            true, // allow_all_groups
            false,
            false,
            false,
            LocationMfaMode::Disabled,
            ServiceLocationMode::Disabled,
        )
        .set_address([IpNetwork::from_str("10.7.1.1/24").unwrap()])
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

        let groups = Vec::new();
        let result = has_non_posture_location_access(&pool, &groups)
            .await
            .unwrap();
        assert!(result);
    }

    /// A location with posture checks assigned should not grant access, even
    /// when it is otherwise accessible to the user.
    #[sqlx::test]
    async fn test_posture_location_returns_false(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let network = WireguardNetwork::new(
            "posture-net".to_owned(),
            50058,
            String::new(),
            None,
            [IpNetwork::from_str("10.8.1.0/24").unwrap()],
            true, // allow_all_groups
            false,
            false,
            false,
            LocationMfaMode::Disabled,
            ServiceLocationMode::Disabled,
        )
        .set_address([IpNetwork::from_str("10.8.1.1/24").unwrap()])
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

        let posture_id: i64 = query_scalar!(
            "INSERT INTO device_posture (name) VALUES ($1) RETURNING id",
            "test-posture"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO device_posture_location (posture_id, location_id) VALUES ($1, $2)",
            posture_id,
            network.id,
        )
        .execute(&pool)
        .await
        .unwrap();

        let groups = Vec::new();
        let result = has_non_posture_location_access(&pool, &groups)
            .await
            .unwrap();
        assert!(!result);
    }

    /// Two networks accessible: one with posture checks, one without = true.
    #[sqlx::test]
    async fn test_mixed_posture_networks_when_one_has_no_posture(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;

        let group = Group::new("qa").save(&pool).await.unwrap();

        // Location without posture checks, restricted to the qa group.
        let no_posture_net = WireguardNetwork::new(
            "qa-net".to_owned(),
            50059,
            String::new(),
            None,
            [IpNetwork::from_str("10.9.1.0/24").unwrap()],
            false,
            false,
            false,
            false,
            LocationMfaMode::Disabled,
            ServiceLocationMode::Disabled,
        )
        .set_address([IpNetwork::from_str("10.9.1.1/24").unwrap()])
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

        // Location with posture checks, accessible to everyone.
        let posture_net = WireguardNetwork::new(
            "posture-net-2".to_owned(),
            50060,
            String::new(),
            None,
            [IpNetwork::from_str("10.10.1.0/24").unwrap()],
            true, // allow_all_groups
            false,
            false,
            false,
            LocationMfaMode::Disabled,
            ServiceLocationMode::Disabled,
        )
        .set_address([IpNetwork::from_str("10.10.1.1/24").unwrap()])
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

        let posture_id: i64 = query_scalar!(
            "INSERT INTO device_posture (name) VALUES ($1) RETURNING id",
            "test-posture-2"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO device_posture_location (posture_id, location_id) VALUES ($1, $2)",
            posture_id,
            posture_net.id,
        )
        .execute(&pool)
        .await
        .unwrap();

        let mut transaction = pool.begin().await.unwrap();
        no_posture_net
            .set_allowed_groups(&mut transaction, from_ref(&group.name))
            .await
            .unwrap();
        transaction.commit().await.unwrap();

        let groups = vec!["qa".to_owned()];
        let result = has_non_posture_location_access(&pool, &groups)
            .await
            .unwrap();
        assert!(result);
    }
}
