use openidconnect::{AuthorizationCode, Nonce};
use reqwest::Url;
use tonic::Status;

use crate::{
    enterprise::{
        handlers::openid_login::{extract_state_data, user_from_claims},
        is_enterprise_enabled,
    },
    events::{BidiRequestContext, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent},
    grpc::{
        desktop_client_mfa::{ClientLoginSession, ClientMfaServer},
        proto::proxy::{ClientMfaOidcAuthenticateRequest, DeviceInfo, MfaMethod},
        utils::parse_client_info,
    },
};

impl ClientMfaServer {
    #[instrument(skip_all)]
    pub async fn auth_mfa_session_with_oidc(
        &mut self,
        request: ClientMfaOidcAuthenticateRequest,
        info: Option<DeviceInfo>,
    ) -> Result<(), Status> {
        debug!("Received OIDC MFA authentication request: {request:?}");
        if !is_enterprise_enabled() {
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
        let pubkey = Self::parse_token(&token)?;

        // fetch login session
        let Some(session) = self.sessions.get(&pubkey).cloned() else {
            debug!("Client login session not found");
            return Err(Status::invalid_argument("login session not found"));
        };
        let ClientLoginSession {
            method,
            device,
            location,
            user,
            openid_auth_completed,
        } = session;

        if openid_auth_completed {
            debug!("Client login session already completed");
            return Err(Status::invalid_argument("login session already completed"));
        }

        if method != MfaMethod::Oidc {
            debug!("Invalid MFA method for OIDC authentication: {method:?}");
            self.sessions.remove(&pubkey);
            return Err(Status::invalid_argument("invalid MFA method"));
        }

        let (ip, user_agent) = parse_client_info(&info).map_err(Status::internal)?;
        let context = BidiRequestContext::new(user.id, user.username.clone(), ip, user_agent);

        let code = AuthorizationCode::new(request.code.clone());
        let url = match Url::parse(&request.callback_url).map_err(|err| {
            error!("Invalid redirect URL provided: {err:?}");
            Status::invalid_argument("invalid redirect URL")
        }) {
            Ok(url) => url,
            Err(status) => {
                self.sessions.remove(&pubkey);
                self.emit_event(BidiStreamEvent {
                    context,
                    event: BidiStreamEventType::DesktopClientMfa(Box::new(
                        DesktopClientMfaEvent::Failed {
                            location: location.clone(),
                            device: device.clone(),
                            method,
                        },
                    )),
                })?;
                return Err(status);
            }
        };

        match user_from_claims(&self.pool, Nonce::new(request.nonce.clone()), code, url).await {
            Ok(claims_user) => {
                // if thats not our user, prevent login
                if claims_user.id != user.id {
                    info!("User {claims_user} tried to use OIDC MFA for another user: {user}");
                    self.sessions.remove(&pubkey);
                    self.emit_event(BidiStreamEvent {
                        context,
                        event: BidiStreamEventType::DesktopClientMfa(Box::new(
                            DesktopClientMfaEvent::Failed {
                                location: location.clone(),
                                device: device.clone(),
                                method,
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
                info!("Failed to verify OIDC code: {err:?}");
                self.sessions.remove(&pubkey);
                self.emit_event(BidiStreamEvent {
                    context,
                    event: BidiStreamEventType::DesktopClientMfa(Box::new(
                        DesktopClientMfaEvent::Failed {
                            location: location.clone(),
                            device: device.clone(),
                            method,
                        },
                    )),
                })?;
                return Err(Status::unauthenticated("unauthorized"));
            }
        };

        self.sessions.insert(
            pubkey.clone(),
            ClientLoginSession {
                method,
                device: device.clone(),
                location: location.clone(),
                user: user.clone(),
                openid_auth_completed: true,
            },
        );

        Ok(())
    }
}
