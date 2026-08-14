use defguard_common::{
    db::models::{
        Device, Settings, User, WireguardNetwork, vpn_client_mfa_session::VpnClientMfaSession,
    },
    types::AuthFlowType,
};
use defguard_proto::{
    client_types::MfaMethod,
    proxy::{ClientMfaOidcAuthenticateRequest, DeviceInfo},
};
use openidconnect::{AuthorizationCode, Nonce};
use tonic::Status;

use crate::{
    enterprise::{
        handlers::openid_login::{extract_state_data, user_from_claims},
        is_business_license_active,
    },
    events::{BidiRequestContext, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent},
    grpc::{proxy::client_mfa::ClientMfaServer, utils::parse_client_ip_agent},
};

impl ClientMfaServer {
    #[instrument(skip_all)]
    pub async fn auth_mfa_session_with_oidc(
        &mut self,
        request: ClientMfaOidcAuthenticateRequest,
        info: Option<DeviceInfo>,
    ) -> Result<(), Status> {
        debug!("Received OIDC MFA authentication request: {request:?}");
        if !is_business_license_active() {
            error!("OIDC MFA method requires enterprise feature to be enabled");
            return Err(Status::invalid_argument("OIDC MFA method is not supported"));
        }

        let token = extract_state_data(&request.state).ok_or_else(|| {
            error!(
                "Failed to extract state data from state: {:?}",
                request.state
            );
            Status::invalid_argument("invalid state data")
        })?;
        if token.is_empty() {
            debug!("Empty token provided in request");
            return Err(Status::invalid_argument("empty token provided"));
        }

        // Fetch the durable in-progress session by the opaque token.
        let Some(session) = VpnClientMfaSession::find_active_by_token(&self.pool, &token).await
        else {
            debug!("Client login session not found");
            return Err(Status::invalid_argument("login session not found"));
        };

        // Fetch the related objects for event context.
        let location = WireguardNetwork::find_by_id(&self.pool, session.location_id)
            .await
            .map_err(|_| Status::internal("unexpected error"))?
            .ok_or_else(|| Status::internal("location not found"))?;
        let device = Device::find_by_id(&self.pool, session.device_id)
            .await
            .map_err(|_| Status::internal("unexpected error"))?
            .ok_or_else(|| Status::internal("device not found"))?;
        let user = User::find_by_id(&self.pool, session.user_id)
            .await
            .map_err(|_| Status::internal("unexpected error"))?
            .ok_or_else(|| Status::internal("user not found"))?;

        // The attempt recorded at start holds the selected method and step attempt id.
        let Some(ephemeral) = session.ephemeral_state.as_ref() else {
            debug!("No MFA attempt in progress");
            return Err(Status::invalid_argument("no MFA attempt in progress"));
        };
        let method: MfaMethod = ephemeral.selected_method.into();
        let step_attempt_id = ephemeral.step_attempt_id.clone();
        let openid_auth_completed = ephemeral.openid_auth_completed;

        if openid_auth_completed {
            debug!("Client login session already completed");
            return Err(Status::invalid_argument("login session already completed"));
        }

        if method != MfaMethod::Oidc {
            debug!("Invalid MFA method for OIDC authentication: {method:?}");
            let mut conn = self.pool.acquire().await.map_err(|_| {
                error!("Failed to acquire DB connection");
                Status::internal("unexpected error")
            })?;
            session.delete(&mut *conn).await.map_err(|err| {
                error!("Failed to delete MFA session: {err}");
                Status::internal("unexpected error")
            })?;
            return Err(Status::invalid_argument("invalid MFA method"));
        }

        let (ip, user_agent) = parse_client_ip_agent(&info).map_err(Status::internal)?;
        let context = BidiRequestContext::new(
            user.id,
            user.username.clone(),
            ip,
            format!("{} (ID {})", device.name, device.id),
        );

        let code = AuthorizationCode::new(request.code.clone());
        let url = match Settings::get_current_settings()
            .edge_callback_url(AuthFlowType::Mfa)
            .map_err(|err| {
                error!("Invalid callback URL configuration: {err}");
                Status::invalid_argument("invalid callback URL")
            }) {
            Ok(url) => url,
            Err(status) => {
                let mut conn = self.pool.acquire().await.map_err(|_| {
                    error!("Failed to acquire DB connection");
                    Status::internal("unexpected error")
                })?;
                session.delete(&mut *conn).await.map_err(|err| {
                    error!("Failed to delete MFA session: {err}");
                    Status::internal("unexpected error")
                })?;
                self.emit_event(BidiStreamEvent {
                    context,
                    event: BidiStreamEventType::DesktopClientMfa(Box::new(
                        DesktopClientMfaEvent::Failed {
                            location: location.clone(),
                            device: device.clone(),
                            method,
                            message: "provided invalid callback URL".to_owned(),
                        },
                    )),
                })?;
                return Err(status);
            }
        };

        // This path only re-verifies an already-existing user's identity via OpenID
        // for MFA, so it never creates a new account, hence no `ApiEvent` channel.
        match user_from_claims(
            &self.pool,
            Nonce::new(request.nonce.clone()),
            code,
            url,
            Some(ip),
            Some(&user_agent),
            None,
        )
        .await
        {
            Ok(claims_user) => {
                // if thats not our user, prevent login
                if claims_user.id != user.id {
                    info!("User {claims_user} tried to use OIDC MFA for another user: {user}");
                    let mut conn = self.pool.acquire().await.map_err(|_| {
                        error!("Failed to acquire DB connection");
                        Status::internal("unexpected error")
                    })?;
                    session.delete(&mut *conn).await.map_err(|err| {
                        error!("Failed to delete MFA session: {err}");
                        Status::internal("unexpected error")
                    })?;
                    self.emit_event(BidiStreamEvent {
                        context,
                        event: BidiStreamEventType::DesktopClientMfa(Box::new(
                            DesktopClientMfaEvent::Failed {
                                location: location.clone(),
                                device: device.clone(),
                                method,
                                message: format!("user {claims_user} tried to use OIDC MFA for another user: {user}")
                            },
                        )),
                    })?;
                    return Err(Status::unauthenticated("unauthorized"));
                }
                info!(
                    "OIDC MFA authentication completed successfully for user: {}",
                    user.username
                );
            }
            Err(err) => {
                info!("Failed to verify OIDC code: {err}");
                let mut conn = self.pool.acquire().await.map_err(|_| {
                    error!("Failed to acquire DB connection");
                    Status::internal("unexpected error")
                })?;
                session.delete(&mut *conn).await.map_err(|err| {
                    error!("Failed to delete MFA session: {err}");
                    Status::internal("unexpected error")
                })?;
                self.emit_event(BidiStreamEvent {
                    context,
                    event: BidiStreamEventType::DesktopClientMfa(Box::new(
                        DesktopClientMfaEvent::Failed {
                            location: location.clone(),
                            device: device.clone(),
                            method,
                            message: format!("failed to verify OIDC code: {err}"),
                        },
                    )),
                })?;
                return Err(Status::unauthenticated("unauthorized"));
            }
        }

        // Mark the OIDC attempt complete. A stale step_attempt_id is a no-op.
        let mut conn = self.pool.acquire().await.map_err(|_| {
            error!("Failed to acquire DB connection");
            Status::internal("unexpected error")
        })?;
        session
            .mark_oidc_completed(&mut conn, &step_attempt_id)
            .await
            .map_err(|err| {
                error!("Failed to mark OIDC attempt complete: {err}");
                Status::internal("unexpected error")
            })?;

        Ok(())
    }
}
