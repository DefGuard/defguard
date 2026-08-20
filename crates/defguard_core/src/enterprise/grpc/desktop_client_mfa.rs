use defguard_common::{
    db::{
        Id,
        models::{
            Settings,
            vpn_client_mfa_session::{MfaSessionContext, VpnClientMfaSession},
        },
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
        handlers::openid_login::{MfaOidcState, extract_state_data, user_from_claims},
        is_business_license_active,
    },
    events::{BidiRequestContext, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent},
    grpc::{proxy::client_mfa::ClientMfaServer, utils::parse_client_ip_agent},
    mfa_engine::authorize::emit_event,
};

impl ClientMfaServer {
    #[instrument(skip_all)]
    pub async fn auth_mfa_session_with_oidc(
        &mut self,
        request: ClientMfaOidcAuthenticateRequest,
        info: Option<DeviceInfo>,
    ) -> Result<(), Status> {
        debug!("Received OIDC MFA authentication request");
        if !is_business_license_active() {
            error!("OIDC MFA method requires enterprise feature to be enabled");
            return Err(Status::invalid_argument("OIDC MFA method is not supported"));
        }

        let state_data = extract_state_data(&request.state).ok_or_else(|| {
            error!("Failed to extract state data from state");
            Status::invalid_argument("invalid state data")
        })?;

        // The MFA flow's state carries "<token>.<step_attempt_id>". A state with no valid
        // attempt id (for example, an OIDC login in flight across the upgrade) is rejected rather
        // than silently accepted.
        let MfaOidcState { token, attempt_id } =
            MfaOidcState::parse(&state_data).ok_or_else(|| {
                debug!("OIDC MFA state carries no valid <token>.<step_attempt_id> payload");
                Status::invalid_argument("invalid state data")
            })?;

        // Fetch the durable in-progress session by the opaque token.
        let Some(session) = VpnClientMfaSession::<Id>::find_active_by_token(&self.pool, &token)
            .await
            .map_err(|err| {
                error!("Failed to find MFA session: {err}");
                Status::internal("unexpected error")
            })?
        else {
            debug!("Client login session not found");
            return Err(Status::invalid_argument("login session not found"));
        };

        // Fetch the related objects for event context.
        let Some(MfaSessionContext {
            location,
            device,
            user,
        }) = session.load_context(&self.pool).await.map_err(|err| {
            error!("Failed to load MFA session context: {err}");
            Status::internal("unexpected error")
        })?
        else {
            error!("MFA session references a missing location, device, or user");
            return Err(Status::internal("unexpected error"));
        };

        // The attempt recorded at start holds the selected method and step attempt id.
        let Some(ephemeral) = session.ephemeral_state.as_ref() else {
            debug!("No MFA attempt in progress");
            return Err(Status::invalid_argument("no MFA attempt in progress"));
        };

        // Bind the callback to the attempt it was issued for before any branch below can mutate
        // or delete the session. Every abort path from here on deletes the row, so a callback
        // belonging to a superseded attempt must be turned away first: otherwise a late callback
        // carrying a stale id could tear down the attempt that replaced it.
        if ephemeral.step_attempt_id != attempt_id {
            debug!("OIDC MFA callback arrived for a superseded attempt");
            return Err(Status::invalid_argument("stale OIDC MFA attempt"));
        }

        let method: MfaMethod = ephemeral.selected_method.into();
        let openid_auth_completed = ephemeral.openid_auth_completed;

        if openid_auth_completed {
            debug!("Client login session already completed");
            return Err(Status::invalid_argument("login session already completed"));
        }

        if method != MfaMethod::Oidc {
            debug!("Invalid MFA method for OIDC authentication: {method:?}");
            self.delete_mfa_session(session).await?;
            return Err(Status::invalid_argument("invalid MFA method"));
        }

        let (ip, user_agent) = parse_client_ip_agent(&info).map_err(|err| {
            error!("Failed to parse client IP and agent during OIDC MFA: {err}");
            Status::internal("unexpected error")
        })?;
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
                self.delete_mfa_session(session).await?;
                emit_event(
                    &self.bidi_event_tx,
                    BidiStreamEvent {
                        context,
                        event: BidiStreamEventType::DesktopClientMfa(Box::new(
                            DesktopClientMfaEvent::Failed {
                                location: location.clone(),
                                device: device.clone(),
                                method,
                                message: "provided invalid callback URL".to_owned(),
                            },
                        )),
                    },
                )?;
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
                    self.delete_mfa_session(session).await?;
                    emit_event(
                        &self.bidi_event_tx,
                        BidiStreamEvent {
                            context,
                            event: BidiStreamEventType::DesktopClientMfa(Box::new(
                                DesktopClientMfaEvent::Failed {
                                    location: location.clone(),
                                    device: device.clone(),
                                    method,
                                    message: format!(
                                        "user {claims_user} tried to use OIDC MFA for another user: {user}"
                                    ),
                                },
                            )),
                        },
                    )?;
                    return Err(Status::unauthenticated("unauthorized"));
                }
                info!(
                    "OIDC MFA authentication completed successfully for user: {}",
                    user.username
                );
            }
            Err(err) => {
                info!("Failed to verify OIDC code: {err}");
                self.delete_mfa_session(session).await?;
                emit_event(
                    &self.bidi_event_tx,
                    BidiStreamEvent {
                        context,
                        event: BidiStreamEventType::DesktopClientMfa(Box::new(
                            DesktopClientMfaEvent::Failed {
                                location: location.clone(),
                                device: device.clone(),
                                method,
                                message: format!("failed to verify OIDC code: {err}"),
                            },
                        )),
                    },
                )?;
                return Err(Status::unauthenticated("unauthorized"));
            }
        }

        // Mark the OIDC attempt complete, gated on the attempt id the state carried. A stale or
        // absent attempt is a no-op (returns false) and must be rejected, not silently accepted.
        // This repeats the check made above against the row read at the start of the request, and
        // deliberately so: the SQL predicate re-evaluates the id at write time, so an attempt
        // re-issued while the OIDC round trip was in flight is caught here rather than marked.
        let mut conn = self.acquire_conn().await?;
        let marked = session
            .mark_oidc_completed(&mut conn, &attempt_id)
            .await
            .map_err(|err| {
                error!("Failed to mark OIDC attempt complete: {err}");
                Status::internal("unexpected error")
            })?;
        if !marked {
            debug!("OIDC MFA callback arrived for a superseded or absent attempt");
            return Err(Status::invalid_argument("stale OIDC MFA attempt"));
        }

        Ok(())
    }

    /// Delete a durable MFA session, mapping database errors to a gRPC status.
    async fn delete_mfa_session(&self, session: VpnClientMfaSession<Id>) -> Result<(), Status> {
        let mut conn = self.acquire_conn().await?;
        session.delete(&mut *conn).await.map_err(|err| {
            error!("Failed to delete MFA session: {err}");
            Status::internal("unexpected error")
        })
    }
}
