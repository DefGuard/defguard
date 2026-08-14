use chrono::{NaiveDateTime, Utc};
use model_derive::Model;
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, Type, query_as};
use utoipa::ToSchema;

use crate::db::{
    Id, NoId,
    models::{
        WireguardNetwork, biometric_auth::BiometricAuth, user::User,
        vpn_session_stats::VpnSessionStats,
    },
};

#[derive(Clone, Debug, Default, PartialEq, Type)]
#[sqlx(type_name = "vpn_client_session_state", rename_all = "lowercase")]
pub enum VpnClientSessionState {
    #[default]
    New,
    Connected,
    Disconnected,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, ToSchema, Type)]
#[sqlx(type_name = "vpn_client_mfa_method", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum VpnClientMfaMethod {
    Totp,
    Email,
    Oidc,
    Biometric,
    MobileApprove,
}

impl VpnClientMfaMethod {
    /// Returns whether this method is configured for `user` (and, for biometric, `device_id`).
    ///
    /// Per-user/per-device setup state is ANDed with deployment-level availability:
    /// `smtp_configured` gates email and `oidc_configured` gates OIDC. The remaining methods have
    /// no deployment gate. The caller computes `smtp_configured` and `oidc_configured` (the latter
    /// including license tier and provider presence), so this predicate stays model-level.
    ///
    /// Per-user setup reads `User::totp_enabled` (TOTP), `User::email_mfa_enabled` (email),
    /// `User::openid_sub` (OIDC identity), and the `biometric_auth` table - keyed on the device
    /// for biometric and on any of the user's devices for mobile-approve.
    pub async fn is_configured<'e, E: PgExecutor<'e>>(
        self,
        executor: E,
        user: &User<Id>,
        device_id: Id,
        smtp_configured: bool,
        oidc_configured: bool,
    ) -> sqlx::Result<bool> {
        let configured = match self {
            Self::Totp => user.totp_enabled,
            Self::Email => smtp_configured && user.email_mfa_enabled,
            Self::Oidc => oidc_configured && user.openid_sub.is_some(),
            Self::Biometric => BiometricAuth::find_by_device_id(executor, device_id)
                .await?
                .is_some(),
            Self::MobileApprove => !BiometricAuth::find_by_user_id(executor, user.id)
                .await?
                .is_empty(),
        };
        Ok(configured)
    }
}

/// Represents a single VPN client session from creation to eventual disconnection
#[derive(Debug, Model)]
#[table(vpn_client_session)]
pub struct VpnClientSession<I = NoId> {
    pub id: I,
    pub location_id: Id,
    pub user_id: Id,
    pub device_id: Id,
    pub created_at: NaiveDateTime,
    pub connected_at: Option<NaiveDateTime>,
    pub disconnected_at: Option<NaiveDateTime>,
    #[model(option)]
    pub mfa_method: Option<VpnClientMfaMethod>,
    #[model(enum)]
    pub state: VpnClientSessionState,
    pub preshared_key: Option<String>,
}

impl VpnClientSession {
    #[must_use]
    pub fn new(
        location_id: Id,
        user_id: Id,
        device_id: Id,
        connected_at: Option<NaiveDateTime>,
        mfa_method: Option<VpnClientMfaMethod>,
    ) -> Self {
        // determine session state
        let state = if connected_at.is_some() {
            VpnClientSessionState::Connected
        } else {
            VpnClientSessionState::New
        };

        Self {
            id: NoId,
            location_id,
            user_id,
            device_id,
            created_at: Utc::now().naive_utc(),
            connected_at,
            disconnected_at: None,
            mfa_method,
            state,
            preshared_key: None,
        }
    }
}

impl VpnClientSession<Id> {
    /// Tries to fetch the latest active session for a given location and device
    ///
    /// A session is considered active if it's state is `New` or `Connected`
    pub async fn try_get_active_session<'e, E: sqlx::PgExecutor<'e>>(
        executor: E,
        location_id: Id,
        device_id: Id,
    ) -> sqlx::Result<Option<Self>> {
        query_as!(
            Self,
            "SELECT id, location_id, user_id, device_id, created_at, connected_at, disconnected_at, \
	            mfa_method \"mfa_method: VpnClientMfaMethod\", state \"state: VpnClientSessionState\", preshared_key \
			FROM vpn_client_session \
			WHERE location_id = $1 AND device_id = $2 AND state IN ('new', 'connected') \
			ORDER BY created_at DESC, id DESC \
			LIMIT 1",
            location_id,
            device_id
        )
        .fetch_optional(executor)
        .await
    }

    /// Returns latest stats in a given session for each gateway
    pub async fn get_latest_stats_for_all_gateways<'e, E: sqlx::PgExecutor<'e>>(
        &self,
        executor: E,
    ) -> sqlx::Result<Vec<VpnSessionStats<Id>>> {
        query_as!(
            VpnSessionStats,
            "SELECT DISTINCT ON (gateway_id) id, session_id, gateway_id, collected_at, latest_handshake, endpoint, \
            	total_upload, total_download, upload_diff, download_diff
        	FROM vpn_session_stats \
        	WHERE session_id = $1 \
        	ORDER BY gateway_id, collected_at DESC",
            self.id
        )
        .fetch_all(executor)
        .await
    }

    /// Fetch active sessions which have become inactive for a specific location
    pub async fn get_all_inactive_for_location<'e, E: sqlx::PgExecutor<'e>>(
        executor: E,
        location: &WireguardNetwork<Id>,
    ) -> sqlx::Result<Vec<Self>> {
        query_as!(
    		Self,
            "SELECT s.id, location_id, user_id, device_id, created_at, s.connected_at, disconnected_at, \
	            mfa_method \"mfa_method: VpnClientMfaMethod\", state \"state: VpnClientSessionState\", preshared_key \
			FROM vpn_client_session s \
			LEFT JOIN LATERAL ( \
				SELECT latest_handshake \
				FROM vpn_session_stats \
				WHERE session_id = s.id \
				ORDER BY latest_handshake DESC \
				LIMIT 1 \
			) ss ON true \
			WHERE location_id = $1 AND state = 'connected' \
            AND (NOW() - ss.latest_handshake) > $2 * interval '1 second'",
			location.id,
			f64::from(location.peer_disconnect_threshold)
    	).fetch_all(executor).await
    }

    /// Fetch sessions that were created but have not become `connected` within the disconnect threshold
    pub async fn get_never_connected<'e, E: sqlx::PgExecutor<'e>>(
        executor: E,
        location: &WireguardNetwork<Id>,
    ) -> sqlx::Result<Vec<Self>> {
        query_as!(
    		Self,
            "SELECT id, location_id, user_id, device_id, created_at, connected_at, disconnected_at, \
	            mfa_method \"mfa_method: VpnClientMfaMethod\", state \"state: VpnClientSessionState\", preshared_key \
			FROM vpn_client_session \
			WHERE location_id = $1 AND state = 'new' \
            AND (NOW() - created_at) > $2 * interval '1 second'",
			location.id,
			f64::from(location.peer_disconnect_threshold)
    	).fetch_all(executor).await
    }

    /// Fetch all active sessions for a given device in a given location
    pub async fn get_all_active_device_sessions_in_location<'e, E: sqlx::PgExecutor<'e>>(
        executor: E,
        location_id: Id,
        device_id: Id,
    ) -> sqlx::Result<Vec<Self>> {
        query_as!(
    		Self,
            "SELECT id, location_id, user_id, device_id, created_at, connected_at, disconnected_at, \
	            mfa_method \"mfa_method: VpnClientMfaMethod\", state \"state: VpnClientSessionState\", preshared_key \
			FROM vpn_client_session \
			WHERE location_id = $1 AND device_id = $2 AND state IN ('new', 'connected') \
			ORDER BY created_at DESC, id DESC",
			location_id,
			device_id,
    	).fetch_all(executor).await
    }
}

#[cfg(test)]
mod tests {
    use sqlx::{
        PgPool,
        postgres::{PgConnectOptions, PgPoolOptions},
    };

    use super::VpnClientMfaMethod;
    use crate::db::{
        Id,
        models::{Device, DeviceType, User, biometric_auth::BiometricAuth},
        setup_pool,
    };

    async fn create_user(pool: &PgPool) -> User<Id> {
        User::new(
            "mfa-configured-test",
            None,
            "Test",
            "User",
            "mfa-configured@test.example",
            None,
        )
        .save(pool)
        .await
        .expect("failed to create user")
    }

    async fn create_device(pool: &PgPool, user_id: Id) -> Device<Id> {
        Device::new(
            "mfa-configured-device".to_owned(),
            "mfa-configured-pubkey".to_owned(),
            user_id,
            DeviceType::User,
            None,
            true,
        )
        .save(pool)
        .await
        .expect("failed to create device")
    }

    #[sqlx::test]
    async fn test_is_configured(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let mut user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;

        // Nothing set up: every method is unconfigured regardless of deployment availability.
        assert!(
            !VpnClientMfaMethod::Totp
                .is_configured(&pool, &user, device.id, false, false)
                .await
                .unwrap()
        );
        assert!(
            !VpnClientMfaMethod::Email
                .is_configured(&pool, &user, device.id, true, false)
                .await
                .unwrap()
        );
        assert!(
            !VpnClientMfaMethod::Oidc
                .is_configured(&pool, &user, device.id, false, true)
                .await
                .unwrap()
        );
        assert!(
            !VpnClientMfaMethod::Biometric
                .is_configured(&pool, &user, device.id, false, false)
                .await
                .unwrap()
        );
        assert!(
            !VpnClientMfaMethod::MobileApprove
                .is_configured(&pool, &user, device.id, false, false)
                .await
                .unwrap()
        );

        // TOTP: set up -> configured (no deployment gate).
        user.totp_enabled = true;
        assert!(
            VpnClientMfaMethod::Totp
                .is_configured(&pool, &user, device.id, false, false)
                .await
                .unwrap()
        );

        // Email: set up AND SMTP configured -> configured.
        user.email_mfa_enabled = true;
        assert!(
            VpnClientMfaMethod::Email
                .is_configured(&pool, &user, device.id, true, false)
                .await
                .unwrap()
        );
        // Email set up but SMTP not configured -> unconfigured.
        assert!(
            !VpnClientMfaMethod::Email
                .is_configured(&pool, &user, device.id, false, false)
                .await
                .unwrap()
        );

        // OIDC: identity + oidc_configured -> configured.
        user.openid_sub = Some("oidc-sub".to_owned());
        assert!(
            VpnClientMfaMethod::Oidc
                .is_configured(&pool, &user, device.id, false, true)
                .await
                .unwrap()
        );
        // OIDC identity present but oidc_configured false -> unconfigured.
        assert!(
            !VpnClientMfaMethod::Oidc
                .is_configured(&pool, &user, device.id, false, false)
                .await
                .unwrap()
        );
        // OIDC identity absent -> unconfigured even when oidc_configured.
        user.openid_sub = None;
        assert!(
            !VpnClientMfaMethod::Oidc
                .is_configured(&pool, &user, device.id, false, true)
                .await
                .unwrap()
        );

        // Biometric: the device has a registered biometric auth -> configured.
        BiometricAuth::new(device.id, "biometric-pubkey".to_owned())
            .save(&pool)
            .await
            .expect("failed to save biometric auth");
        assert!(
            VpnClientMfaMethod::Biometric
                .is_configured(&pool, &user, device.id, false, false)
                .await
                .unwrap()
        );

        // MobileApprove: the user has a device with a registered biometric auth -> configured.
        assert!(
            VpnClientMfaMethod::MobileApprove
                .is_configured(&pool, &user, device.id, false, false)
                .await
                .unwrap()
        );
    }
}
