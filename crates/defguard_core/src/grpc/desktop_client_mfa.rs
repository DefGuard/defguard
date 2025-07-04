use std::collections::HashMap;

use chrono::Utc;
use sqlx::PgPool;
use thiserror::Error;
use tokio::sync::{
    broadcast::Sender,
    mpsc::{UnboundedSender, error::SendError},
};
use tonic::{Code, Status};

use super::proto::proxy::{
    self, ClientMfaFinishRequest, ClientMfaFinishResponse, ClientMfaStartRequest,
    ClientMfaStartResponse, MfaMethod,
};
use crate::{
    auth::{Claims, ClaimsType},
    db::{
        Device, GatewayEvent, Id, Settings, User, UserInfo, WireguardNetwork,
        models::device::{DeviceInfo, DeviceNetworkInfo, WireguardNetworkDevice},
    },
    enterprise::{db::models::openid_provider::OpenIdProvider, is_enterprise_enabled},
    events::{BidiRequestContext, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent},
    grpc::utils::parse_client_info,
    handlers::mail::send_email_mfa_code_email,
    mail::Mail,
};

const CLIENT_SESSION_TIMEOUT: u64 = 60 * 5; // 10 minutes

#[derive(Debug, Error)]
pub enum ClientMfaServerError {
    #[error("gRPC event channel error: {0}")]
    BidiEventChannelError(#[from] SendError<BidiStreamEvent>),
}

impl From<ClientMfaServerError> for Status {
    fn from(value: ClientMfaServerError) -> Self {
        Self::new(Code::Internal, value.to_string())
    }
}

#[derive(Clone)]
pub(crate) struct ClientLoginSession {
    pub(crate) method: MfaMethod,
    pub(crate) location: WireguardNetwork<Id>,
    pub(crate) device: Device<Id>,
    pub(crate) user: User<Id>,
    pub(crate) openid_auth_completed: bool,
}

pub(crate) struct ClientMfaServer {
    pub(crate) pool: PgPool,
    mail_tx: UnboundedSender<Mail>,
    wireguard_tx: Sender<GatewayEvent>,
    pub(crate) sessions: HashMap<String, ClientLoginSession>,
    bidi_event_tx: UnboundedSender<BidiStreamEvent>,
}

impl ClientMfaServer {
    #[must_use]
    pub fn new(
        pool: PgPool,
        mail_tx: UnboundedSender<Mail>,
        wireguard_tx: Sender<GatewayEvent>,
        bidi_event_tx: UnboundedSender<BidiStreamEvent>,
    ) -> Self {
        Self {
            pool,
            mail_tx,
            wireguard_tx,
            bidi_event_tx,
            sessions: HashMap::new(),
        }
    }

    fn generate_token(pubkey: &str) -> Result<String, Status> {
        Claims::new(
            ClaimsType::DesktopClient,
            String::new(),
            pubkey.into(),
            CLIENT_SESSION_TIMEOUT,
        )
        .to_jwt()
        .map_err(|err| {
            error!("Failed to generate JWT token: {err:?}");
            Status::internal("unexpected error")
        })
    }

    /// Validate JWT and extract client pubkey
    pub(crate) fn parse_token(token: &str) -> Result<String, Status> {
        let claims = Claims::from_jwt(ClaimsType::DesktopClient, token).map_err(|err| {
            error!("Failed to parse JWT token: {err:?}");
            Status::invalid_argument("invalid token")
        })?;
        Ok(claims.client_id)
    }

    pub(crate) fn emit_event(&self, event: BidiStreamEvent) -> Result<(), ClientMfaServerError> {
        Ok(self.bidi_event_tx.send(event)?)
    }

    #[instrument(skip_all)]
    pub async fn start_client_mfa_login(
        &mut self,
        request: ClientMfaStartRequest,
    ) -> Result<ClientMfaStartResponse, Status> {
        debug!("Starting desktop client login: {request:?}");
        // fetch location
        let Ok(Some(location)) =
            WireguardNetwork::find_by_id(&self.pool, request.location_id).await
        else {
            error!("Failed to find location with ID {}", request.location_id);
            return Err(Status::invalid_argument("location not found"));
        };

        // fetch device
        let Ok(Some(device)) = Device::find_by_pubkey(&self.pool, &request.pubkey).await else {
            error!("Failed to find device with pubkey {}", request.pubkey);
            return Err(Status::invalid_argument("device not found"));
        };

        // fetch user
        let Ok(Some(mut user)) = User::find_by_id(&self.pool, device.user_id).await else {
            error!("Failed to find user with ID {}", device.user_id);
            return Err(Status::invalid_argument("user not found"));
        };
        let user_info = UserInfo::from_user(&self.pool, &user).await.map_err(|_| {
            error!("Failed to fetch user info for {}", user.username);
            Status::internal("unexpected error")
        })?;

        // validate user is allowed to connect to a given location
        let mut transaction = self.pool.begin().await.map_err(|_| {
            error!("Failed to begin transaction");
            Status::internal("unexpected error")
        })?;
        let allowed_groups = location
            .get_allowed_groups(&mut transaction)
            .await
            .map_err(|err| {
                error!("Failed to fetch allowed groups for location {location}: {err:?}");
                Status::internal("unexpected error")
            })?;
        if let Some(groups) = allowed_groups {
            // check if user belongs to one of allowed groups
            if !groups
                .iter()
                .any(|allowed_group| user_info.groups.contains(allowed_group))
            {
                error!(
                    "User {} not allowed to connect to location {location} because he doesn't belong to any of the allowed groups.
                    User groups: {:?}, allowed groups: {:?}",
                    user.username, user_info.groups, groups
                );
                return Err(Status::unauthenticated("unauthorized"));
            }
        }

        user.verify_mfa_state(&self.pool).await.map_err(|err| {
            error!(
                "Failed to verify MFA state for user {}: {err:?}",
                user.username
            );
            Status::internal("unexpected error")
        })?;

        // check if selected method is enabled
        let method = MfaMethod::try_from(request.method).map_err(|err| {
            error!("Invalid MFA method selected ({}): {err}", request.method);
            Status::invalid_argument("invalid MFA method selected")
        })?;
        match method {
            MfaMethod::Totp => {
                if !user.totp_enabled {
                    error!("TOTP not enabled for user {}", user.username);
                    return Err(Status::invalid_argument(
                        "selected MFA method not available",
                    ));
                }
            }
            MfaMethod::Email => {
                if !user.email_mfa_enabled {
                    error!("Email MFA not enabled for user {}", user.username);
                    return Err(Status::invalid_argument(
                        "selected MFA method not available",
                    ));
                }
                // send email code
                send_email_mfa_code_email(&user, &self.mail_tx, None).map_err(|err| {
                    error!(
                        "Failed to send email MFA code for user {}: {err:?}",
                        user.username
                    );
                    Status::internal("unexpected error")
                })?;
            }
            MfaMethod::Oidc => {
                if !is_enterprise_enabled() {
                    error!("OIDC MFA method requires enterprise feature to be enabled");
                    return Err(Status::invalid_argument(
                        "selected MFA method not available",
                    ));
                }

                let settings = Settings::get_current_settings();
                if !settings.use_openid_for_mfa {
                    error!("OIDC MFA method is not enabled in settings");
                    return Err(Status::invalid_argument(
                        "selected MFA method not available",
                    ));
                }

                if OpenIdProvider::get_current(&self.pool)
                    .await
                    .map_err(|err| {
                        error!("Failed to get current OpenID provider: {err:?}",);
                        Status::internal("unexpected error")
                    })?
                    .is_none()
                {
                    error!("OIDC provider is not configured");
                    return Err(Status::invalid_argument(
                        "selected MFA method not available",
                    ));
                }
            }
        };

        // generate auth token
        let token = Self::generate_token(&request.pubkey)?;

        info!(
            "Desktop client MFA login started for {} at location {}",
            user.username, location.name
        );

        // store login session
        self.sessions.insert(
            request.pubkey,
            ClientLoginSession {
                method,
                location,
                device,
                user,
                openid_auth_completed: false,
            },
        );

        Ok(ClientMfaStartResponse { token })
    }

    #[instrument(skip_all)]
    pub async fn finish_client_mfa_login(
        &mut self,
        request: ClientMfaFinishRequest,
        info: Option<proxy::DeviceInfo>,
    ) -> Result<ClientMfaFinishResponse, Status> {
        debug!("Finishing desktop client login: {request:?}");
        // get pubkey from token
        let pubkey = Self::parse_token(&request.token)?;

        // fetch login session
        let Some(session) = self.sessions.get(&pubkey) else {
            error!("Client login session not found");
            return Err(Status::invalid_argument("login session not found"));
        };
        let ClientLoginSession {
            method,
            device,
            location,
            user,
            openid_auth_completed,
        } = session;

        // Prepare event context
        let (ip, user_agent) = parse_client_info(&info).map_err(Status::internal)?;
        let context = BidiRequestContext::new(user.id, user.username.clone(), ip, user_agent);

        // validate code
        match method {
            MfaMethod::Totp => {
                let code = if let Some(code) = request.code {
                    code.to_string()
                } else {
                    error!("TOTP code not provided in request");
                    self.emit_event(BidiStreamEvent {
                        context,
                        event: BidiStreamEventType::DesktopClientMfa(Box::new(
                            DesktopClientMfaEvent::Failed {
                                location: location.clone(),
                                device: device.clone(),
                                method: *method,
                                message: "TOTP code not provided in request".to_string(),
                            },
                        )),
                    })?;
                    return Err(Status::invalid_argument("TOTP code not provided"));
                };
                if !user.verify_totp_code(&code) {
                    error!("Provided TOTP code is not valid");
                    self.emit_event(BidiStreamEvent {
                        context,
                        event: BidiStreamEventType::DesktopClientMfa(Box::new(
                            DesktopClientMfaEvent::Failed {
                                location: location.clone(),
                                device: device.clone(),
                                method: *method,
                                message: "invalid TOTP code".to_string(),
                            },
                        )),
                    })?;
                    return Err(Status::unauthenticated("unauthorized"));
                }
            }
            MfaMethod::Email => {
                let code = if let Some(code) = request.code {
                    code.to_string()
                } else {
                    error!("Email MFA code not provided in request");
                    self.emit_event(BidiStreamEvent {
                        context,
                        event: BidiStreamEventType::DesktopClientMfa(Box::new(
                            DesktopClientMfaEvent::Failed {
                                location: location.clone(),
                                device: device.clone(),
                                method: *method,
                                message: "email MFA code not provided in request".to_string(),
                            },
                        )),
                    })?;
                    return Err(Status::invalid_argument("email MFA code not provided"));
                };
                if !user.verify_email_mfa_code(&code) {
                    error!("Provided email code is not valid");
                    self.emit_event(BidiStreamEvent {
                        context,
                        event: BidiStreamEventType::DesktopClientMfa(Box::new(
                            DesktopClientMfaEvent::Failed {
                                location: location.clone(),
                                device: device.clone(),
                                method: *method,
                                message: "invalid email MFA code".to_string(),
                            },
                        )),
                    })?;
                    return Err(Status::unauthenticated("unauthorized"));
                }
            }
            MfaMethod::Oidc => {
                if !*openid_auth_completed {
                    debug!(
                        "User {user} tried to finish OIDC MFA login but they haven't completed the OIDC authentication yet."
                    );
                    self.emit_event(BidiStreamEvent {
                        context,
                        event: BidiStreamEventType::DesktopClientMfa(Box::new(
                            DesktopClientMfaEvent::Failed {
                                location: location.clone(),
                                device: device.clone(),
                                method: *method,
                                message: format!("tried to finish OIDC MFA login but they haven't completed OIDC authentication yet")
                            },
                        )),
                    })?;
                    return Err(Status::failed_precondition(
                        "OIDC authentication not completed yet",
                    ));
                } else {
                    debug!(
                        "User {user} is trying to finish OIDC MFA login and the OIDC authentication has already been completed; proceeding."
                    );
                }
            }
        }

        // begin transaction
        let mut transaction = self.pool.begin().await.map_err(|_| {
            error!("Failed to begin transaction");
            Status::internal("unexpected error")
        })?;

        // fetch device config for the location
        let Ok(Some(mut network_device)) =
            WireguardNetworkDevice::find(&mut *transaction, device.id, location.id).await
        else {
            error!("Failed to fetch network config for device {device} and location {location}");
            return Err(Status::internal("unexpected error"));
        };

        // generate PSK
        let key = WireguardNetwork::genkey();
        network_device.preshared_key = Some(key.public.clone());

        // authorize device for given location
        network_device.is_authorized = true;
        network_device.authorized_at = Some(Utc::now().naive_utc());

        // save updated network config
        network_device
            .update(&mut *transaction)
            .await
            .map_err(|err| {
                error!("Failed to update device network config {network_device:?}: {err:?}");
                Status::internal("unexpected error")
            })?;

        // send gateway event
        debug!("Sending `peer_create` message to gateway");
        let device_info = DeviceInfo {
            device: device.clone(),
            network_info: vec![DeviceNetworkInfo {
                network_id: location.id,
                device_wireguard_ips: network_device.wireguard_ips,
                preshared_key: network_device.preshared_key,
                is_authorized: network_device.is_authorized,
            }],
        };
        let event = GatewayEvent::DeviceCreated(device_info);
        self.wireguard_tx.send(event).map_err(|err| {
            error!("Error sending WireGuard event: {err}");
            Status::internal("unexpected error")
        })?;

        info!(
            "Desktop client login finished for {} at location {}",
            user.username, location.name
        );
        self.emit_event(BidiStreamEvent {
            context,
            event: BidiStreamEventType::DesktopClientMfa(Box::new(
                DesktopClientMfaEvent::Connected {
                    location: location.clone(),
                    device: device.clone(),
                    method: *method,
                },
            )),
        })?;

        // remove login session from map
        self.sessions.remove(&pubkey);

        // commit transaction
        transaction.commit().await.map_err(|_| {
            error!("Failed to commit transaction while finishing desktop client login.");
            Status::internal("unexpected error")
        })?;

        Ok(ClientMfaFinishResponse {
            preshared_key: key.public,
        })
    }
}
