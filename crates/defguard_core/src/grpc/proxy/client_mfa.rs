use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};

use chrono::Utc;
use defguard_common::{
    auth::claims::{Claims, ClaimsType},
    db::{
        Id,
        models::{
            BiometricAuth, BiometricChallenge, Device, DeviceNetworkInfo, User, WireguardNetwork,
            device::{DeviceInfo, WireguardNetworkDevice},
            vpn_client_session::VpnClientSession,
            wireguard::LocationMfaMode,
        },
    },
    types::user_info::UserInfo,
};
use defguard_mail::Mail;
use defguard_proto::proxy::{
    self, AwaitRemoteMfaFinishRequest, AwaitRemoteMfaFinishResponse, ClientMfaFinishRequest,
    ClientMfaFinishResponse, ClientMfaStartRequest, ClientMfaStartResponse,
    ClientMfaTokenValidationRequest, ClientMfaTokenValidationResponse, CoreResponse, MfaMethod,
    core_response::Payload,
};
use sqlx::PgPool;
use thiserror::Error;
use tokio::{
    sync::{
        broadcast::Sender,
        mpsc::{UnboundedSender, error::SendError},
        oneshot,
    },
    time,
};
use tonic::{Code, Status};

use crate::{
    enterprise::{db::models::openid_provider::OpenIdProvider, is_business_license_active},
    events::{BidiRequestContext, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent},
    grpc::{gateway::events::GatewayEvent, utils::parse_client_ip_agent},
    handlers::mail::send_email_mfa_code_email,
};

const CLIENT_SESSION_TIMEOUT: u64 = 60 * 5; // 10 minutes

// How much time the user has to approve remote MFA with mobile device
const REMOTE_AUTH_TIMEOUT: Duration = Duration::from_secs(60);

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
pub struct ClientLoginSession {
    pub(crate) method: MfaMethod,
    pub(crate) location: WireguardNetwork<Id>,
    pub(crate) device: Device<Id>,
    pub(crate) user: User<Id>,
    pub(crate) openid_auth_completed: bool,
    pub(crate) biometric_challenge: Option<BiometricChallenge>,
}

pub struct ClientMfaServer {
    pub(crate) pool: PgPool,
    mail_tx: UnboundedSender<Mail>,
    wireguard_tx: Sender<GatewayEvent>,
    pub(crate) sessions: Arc<RwLock<HashMap<String, ClientLoginSession>>>,
    remote_mfa_responses: Arc<RwLock<HashMap<String, oneshot::Sender<String>>>>,
    bidi_event_tx: UnboundedSender<BidiStreamEvent>,
}

impl ClientMfaServer {
    #[must_use]
    pub fn new(
        pool: PgPool,
        mail_tx: UnboundedSender<Mail>,
        wireguard_tx: Sender<GatewayEvent>,
        bidi_event_tx: UnboundedSender<BidiStreamEvent>,
        remote_mfa_responses: Arc<RwLock<HashMap<String, oneshot::Sender<String>>>>,
        sessions: Arc<RwLock<HashMap<String, ClientLoginSession>>>,
    ) -> Self {
        Self {
            pool,
            mail_tx,
            wireguard_tx,
            bidi_event_tx,
            remote_mfa_responses,
            sessions,
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
            error!("Failed to generate JWT token: {err}");
            Status::internal("unexpected error")
        })
    }

    /// Validate JWT and extract client pubkey
    pub(crate) fn parse_token(token: &str) -> Result<String, Status> {
        let claims = Claims::from_jwt(ClaimsType::DesktopClient, token).map_err(|err| {
            error!("Failed to parse JWT token: {err}");
            Status::invalid_argument("invalid token")
        })?;
        Ok(claims.client_id)
    }

    pub(crate) fn emit_event(&self, event: BidiStreamEvent) -> Result<(), ClientMfaServerError> {
        Ok(self.bidi_event_tx.send(event)?)
    }

    /// Allows proxy to verify if token is valid and active
    #[instrument(skip_all)]
    pub async fn validate_mfa_token(
        &mut self,
        request: ClientMfaTokenValidationRequest,
    ) -> Result<ClientMfaTokenValidationResponse, Status> {
        let pubkey = Self::parse_token(&request.token)?;
        let session_active = self
            .sessions
            .read()
            .expect("Failed to read-lock ClientMfaServer::sessions")
            .contains_key(&pubkey);
        Ok(ClientMfaTokenValidationResponse {
            token_valid: session_active,
        })
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

        // return early if MFA is not enabled for this location
        if !location.mfa_enabled() {
            error!("MFA is not enabled for location {location}");
            return Err(Status::invalid_argument("MFA not enabled for location"));
        }

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
        Self::validate_location_access(&self.pool, &location, &user_info).await?;

        user.verify_mfa_state(&self.pool).await.map_err(|err| {
            error!(
                "Failed to verify MFA state for user {}: {err}",
                user.username
            );
            Status::internal("unexpected error")
        })?;

        // extract user selected method from request
        let selected_method = MfaMethod::try_from(request.method).map_err(|err| {
            error!("Invalid MFA method selected ({}): {err}", request.method);
            Status::invalid_argument("invalid MFA method selected")
        })?;

        // check if selected MFA method matches location settings
        match (&location.location_mfa_mode, selected_method) {
            // MFA enabled status is already verified
            (LocationMfaMode::Disabled, _) => unreachable!(),
            (
                LocationMfaMode::Internal,
                MfaMethod::Totp
                | MfaMethod::Email
                | MfaMethod::Biometric
                | MfaMethod::MobileApprove,
            ) => {
                debug!("Location uses internal MFA. Selected method: {selected_method}");
            }
            (LocationMfaMode::External, MfaMethod::Oidc) => {
                debug!("Location uses external MFA. Selected method: {selected_method}");
            }
            _ => {
                error!(
                    "Selected MFA method ({selected_method}) is not supported by location \
                    {location} which uses {}",
                    location.location_mfa_mode
                );

                return Err(Status::invalid_argument(
                    "selected MFA method not supported by location",
                ));
            }
        }

        let mut selected_mobile_auth: Option<BiometricAuth<Id>> = None;

        // check if selected method is configured
        match selected_method {
            MfaMethod::Biometric => {
                if let Some(found) = BiometricAuth::find_by_device_id(&self.pool, device.id)
                    .await
                    .map_err(|_| Status::internal("unexpected_error"))?
                {
                    selected_mobile_auth = Some(found);
                } else {
                    return Err(Status::invalid_argument(
                        "Select MFA method not available for the device.",
                    ));
                }
            }
            // just check if the account has any devices with biometric auth present
            MfaMethod::MobileApprove => {
                let result = BiometricAuth::find_by_user_id(&self.pool, user.id)
                    .await
                    .map_err(|_| Status::internal("unexpected error"))?;
                if result.is_empty() {
                    return Err(Status::invalid_argument(
                        "selected MFA method not available",
                    ));
                }
            }
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
                        "Failed to send email MFA code for user {}: {err}",
                        user.username
                    );
                    Status::internal("unexpected error")
                })?;
            }
            MfaMethod::Oidc => {
                if !is_business_license_active() {
                    error!("OIDC MFA method requires enterprise feature to be enabled");
                    return Err(Status::invalid_argument(
                        "selected MFA method not available",
                    ));
                }

                if OpenIdProvider::get_current(&self.pool)
                    .await
                    .map_err(|err| {
                        error!("Failed to get current OpenID provider: {err}",);
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
        }

        // generate auth token
        let token = Self::generate_token(&request.pubkey)?;

        info!(
            "Desktop client MFA login started for {} at location {}",
            user.username, location.name
        );

        let biometric_challenge: Option<BiometricChallenge> = match selected_method {
            MfaMethod::Biometric => match selected_mobile_auth {
                Some(mobile_auth) => {
                    let challenge = BiometricChallenge::new_with_owner(&mobile_auth.pub_key).map_err(|e| {
                        error!(
                            "Start biometric mfa failed ! Challenge creation failed ! Reason: {e}"
                        );
                        Status::invalid_argument("Invalid public key")
                    })?;
                    Some(challenge)
                }
                None => {
                    return Err(Status::internal("unexpected error"));
                }
            },
            MfaMethod::MobileApprove => Some(BiometricChallenge::new()),
            _ => None,
        };

        let response_challenge = biometric_challenge
            .as_ref()
            .map(|challenge| challenge.challenge.clone());

        // store login session
        self.sessions
            .write()
            .expect("Failed to write-lock ClientMfaServer::sessions")
            .insert(
                request.pubkey,
                ClientLoginSession {
                    method: selected_method,
                    location,
                    device,
                    user,
                    openid_auth_completed: false,
                    biometric_challenge,
                },
            );

        Ok(ClientMfaStartResponse {
            token,
            challenge: response_challenge,
        })
    }

    /// Checks if given user is allowed to access a location
    async fn validate_location_access(
        pool: &PgPool,
        location: &WireguardNetwork<Id>,
        user_info: &UserInfo,
    ) -> Result<(), Status> {
        // acquire connection
        let mut conn = pool.acquire().await.map_err(|_| {
            error!("Failed to acquire DB connection");
            Status::internal("unexpected error")
        })?;

        // fetch allowed group names for a given location
        let allowed_groups = location
            .get_allowed_groups(&mut conn)
            .await
            .map_err(|err| {
                error!("Failed to fetch allowed groups for location {location}: {err}");
                Status::internal("unexpected error")
            })?;
        // if no groups are specified all users are allowed
        if let Some(groups) = allowed_groups {
            // check if user belongs to one of allowed groups
            if !groups
                .iter()
                .any(|allowed_group| user_info.groups.contains(allowed_group))
            {
                error!(
                    "User {} not allowed to connect to location {location} because he doesn't belong to any of the allowed groups.
                    User groups: {:?}, allowed groups: {:?}",
                    user_info.username, user_info.groups, groups
                );
                return Err(Status::unauthenticated("unauthorized"));
            }
        }
        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn await_remote_mfa_login(
        &mut self,
        request: AwaitRemoteMfaFinishRequest,
        response_tx: UnboundedSender<CoreResponse>,
        request_id: u64,
    ) -> Result<(), Status> {
        debug!("Finishing desktop client login: {request:?}");
        let (tx, rx) = oneshot::channel();
        self.remote_mfa_responses
            .write()
            .expect("Failed to write-lock ClientMfaServer::remote_mfa_responses")
            .insert(request.token.clone(), tx);

        // Spawn a task that waits for remote MFA process to conclude to get the preshared key.
        tokio::spawn(async move {
            match time::timeout(REMOTE_AUTH_TIMEOUT, rx).await {
                Ok(Ok(preshared_key)) => {
                    let req = CoreResponse {
                        id: request_id,
                        payload: Some(Payload::AwaitRemoteMfaFinish(
                            AwaitRemoteMfaFinishResponse { preshared_key },
                        )),
                    };
                    // Once the key is here, send it back to proxy.
                    let _ = response_tx.send(req);
                }
                Ok(Err(err)) => {
                    error!("Remote MFA response channel failed: {err:?}");
                }
                Err(_) => {
                    warn!("Remote MFA process with request_id {request_id} timed out");
                }
            }
        });

        Ok(())
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
        let Some(session) = self
            .sessions
            .read()
            .expect("Failed to read-lock ClientMfaServer::sessions")
            .get(&pubkey)
            .cloned()
        else {
            error!("Client login session not found");
            return Err(Status::invalid_argument("login session not found"));
        };
        let ClientLoginSession {
            method,
            device,
            location,
            user,
            openid_auth_completed,
            biometric_challenge,
        } = session;

        // Prepare event context
        let (ip, _user_agent) = parse_client_ip_agent(&info).map_err(Status::internal)?;
        let context = BidiRequestContext::new(
            user.id,
            user.username.clone(),
            ip,
            format!("{} (ID {})", device.name, device.id),
        );

        // validate code
        match method {
            MfaMethod::MobileApprove => {
                let challenge = biometric_challenge.as_ref().ok_or_else(|| {
                    error!("Challenge not found in MFA session.");
                    Status::invalid_argument("Challenge not found in session")
                })?;
                let signature = request.code.ok_or_else(|| {
                    error!("Signed challenge not found in request");
                    Status::invalid_argument("Signature not found in request")
                })?;
                let auth_device_pub_key = request.auth_pub_key.ok_or_else(|| {
                    Status::invalid_argument("Authorization device key missing in request")
                })?;
                if !BiometricAuth::verify_owner(&self.pool, user.id, &auth_device_pub_key)
                    .await
                    .map_err(|_| Status::internal("unexpected error"))?
                {
                    return Err(Status::invalid_argument("Arguments invalid"));
                }
                match challenge.verify(signature.as_str(), Some(auth_device_pub_key)) {
                    Ok(()) => {
                        debug!("Signature verified successfully.");
                    }
                    Err(err) => {
                        error!(
                            "Verification of challenge for device {} failed; reason {err}",
                            &device.name
                        );
                        self.emit_event(BidiStreamEvent {
                            context,
                            event: BidiStreamEventType::DesktopClientMfa(Box::new(
                                DesktopClientMfaEvent::Failed {
                                    location: location.clone(),
                                    device: device.clone(),
                                    method,
                                    message: "Signed challenge rejected".to_string(),
                                },
                            )),
                        })?;
                        return Err(Status::unauthenticated("unauthorized"));
                    }
                }
            }
            MfaMethod::Biometric => {
                let challenge = biometric_challenge.as_ref().ok_or_else(|| {
                    error!("Challenge not found in MFA session !");
                    Status::internal("Challenge not found in MFA session")
                })?;
                let signed_challenge = request.code.ok_or_else(|| {
                    error!("Signed challenge not found in request");
                    Status::invalid_argument("Challenge not found in request")
                })?;
                match challenge.verify(signed_challenge.as_str(), None) {
                    // verification passed
                    Ok(()) => {
                        debug!("Signature verified successfully.");
                    }
                    // challenge rejected
                    Err(e) => {
                        error!(
                            "Verification of challenge for device {0} failed ! Reason {e}",
                            &device.name
                        );
                        self.emit_event(BidiStreamEvent {
                            context,
                            event: BidiStreamEventType::DesktopClientMfa(Box::new(
                                DesktopClientMfaEvent::Failed {
                                    location: location.clone(),
                                    device: device.clone(),
                                    method,
                                    message: "Signed challenge rejected".to_string(),
                                },
                            )),
                        })?;
                        return Err(Status::unauthenticated("unauthorized"));
                    }
                }
            }
            MfaMethod::Totp => {
                let code = if let Some(code) = request.code {
                    code.clone()
                } else {
                    error!("TOTP code not provided in request");
                    self.emit_event(BidiStreamEvent {
                        context,
                        event: BidiStreamEventType::DesktopClientMfa(Box::new(
                            DesktopClientMfaEvent::Failed {
                                location: location.clone(),
                                device: device.clone(),
                                method,
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
                                method,
                                message: "invalid TOTP code".to_string(),
                            },
                        )),
                    })?;
                    return Err(Status::unauthenticated("unauthorized"));
                }
            }
            MfaMethod::Email => {
                let code = if let Some(code) = request.code {
                    code.clone()
                } else {
                    error!("Email MFA code not provided in request");
                    self.emit_event(BidiStreamEvent {
                        context,
                        event: BidiStreamEventType::DesktopClientMfa(Box::new(
                            DesktopClientMfaEvent::Failed {
                                location: location.clone(),
                                device: device.clone(),
                                method,
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
                                method,
                                message: "invalid email MFA code".to_string(),
                            },
                        )),
                    })?;
                    return Err(Status::unauthenticated("unauthorized"));
                }
            }
            MfaMethod::Oidc => {
                if !openid_auth_completed {
                    debug!(
                        "User {user} tried to finish OIDC MFA login but they haven't completed \
                        the OIDC authentication yet."
                    );
                    self.emit_event(BidiStreamEvent {
                        context,
                        event: BidiStreamEventType::DesktopClientMfa(Box::new(
                            DesktopClientMfaEvent::Failed {
                                location: location.clone(),
                                device: device.clone(),
                                method,
                                message: "tried to finish OIDC MFA login but they haven't \
                                    completed OIDC authentication yet"
                                    .to_string(),
                            },
                        )),
                    })?;
                    return Err(Status::failed_precondition(
                        "OIDC authentication not completed yet",
                    ));
                }
                debug!(
                    "User {user} is trying to finish OIDC MFA login and the OIDC authentication \
                    has already been completed; proceeding."
                );
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
                error!("Failed to update device network config {network_device:?}: {err}");
                Status::internal("unexpected error")
            })?;

        // send gateway event
        debug!("Sending `peer_create` message to gateway");
        let device_info = DeviceInfo {
            device: device.clone(),
            network_info: vec![DeviceNetworkInfo {
                network_id: location.id,
                device_wireguard_ips: network_device.wireguard_ips,
                preshared_key: network_device.preshared_key.clone(),
                is_authorized: network_device.is_authorized,
            }],
        };
        let event = GatewayEvent::DeviceCreated(device_info);
        self.wireguard_tx.send(event).map_err(|err| {
            error!("Error sending WireGuard event: {err}");
            Status::internal("unexpected error")
        })?;

        info!(
            "Desktop client login finished for {} at location {} with method {}",
            user.username,
            location.name,
            method.as_str_name()
        );
        self.emit_event(BidiStreamEvent {
            context,
            event: BidiStreamEventType::DesktopClientMfa(Box::new(
                DesktopClientMfaEvent::Connected {
                    location: location.clone(),
                    device: device.clone(),
                    method,
                },
            )),
        })?;

        // create new VPN client session
        let vpn_client_session = VpnClientSession::new(
            location.id,
            user.id,
            device.id,
            None,
            Some(method.into()),
        )
        .save(&mut *transaction)
            .await
            .map_err(|err| {
                error!("Failed to create new VPN client session for device {device} in location {location}: {err}");
                Status::internal("unexpected error")
            })?;
        debug!("Created new VPN client session: {vpn_client_session:?}");

        let response = ClientMfaFinishResponse {
            preshared_key: key.public,
            token: match method {
                MfaMethod::MobileApprove => Some(request.token.clone()),
                _ => None,
            },
        };

        // remove login session from map
        self.sessions
            .write()
            .expect("Failed to write-lock ClientMfaServer::sessions")
            .remove(&pubkey);

        // commit transaction
        transaction.commit().await.map_err(|_| {
            error!("Failed to commit transaction while finishing desktop client login.");
            Status::internal("unexpected error")
        })?;

        // If there is a desktop client websocket waiting for the preshared key, send it.
        if let (Some(tx), Some(ref preshared_key)) = (
            self.remote_mfa_responses
                .write()
                .expect("Failed to write-lock ClientMfaServer::remote_mfa_responses")
                .remove(&request.token),
            network_device.preshared_key,
        ) {
            let _ = tx.send(preshared_key.clone());
        }

        Ok(response)
    }
}
