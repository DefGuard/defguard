use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, RwLock},
    time::Duration,
};

use chrono::Utc;
use defguard_common::{
    auth::claims::{Claims, ClaimsType},
    db::{
        Id,
        models::{
            BiometricAuth, BiometricChallenge, Device, User, WireguardNetwork,
            device::{DeviceNetworkInfo, WireguardNetworkDevice},
            mfa_flow::MfaFlow,
            polling_token::PollingToken,
            vpn_client_session::{VpnClientMfaMethod, VpnClientSession, VpnClientSessionState},
            wireguard::LocationMfaMode,
        },
    },
    types::user_info::UserInfo,
};
use defguard_proto::{
    client_types::{
        ClientMfaFinishRequest, ClientMfaFinishResponse, ClientMfaStartRequest,
        ClientMfaStartResponse, MfaMethod,
    },
    enterprise::posture::DevicePostureCheckRequest,
    proxy::{
        self, AwaitRemoteMfaFinishRequest, AwaitRemoteMfaFinishResponse,
        ClientMfaTokenValidationRequest, ClientMfaTokenValidationResponse, CoreResponse,
        core_response::Payload,
    },
};
use sqlx::{PgConnection, PgPool};
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
    enterprise::{
        db::models::openid_provider::OpenIdProvider,
        is_business_license_active,
        posture::{PostureCheckError, PostureResult, validate_posture},
    },
    events::{BidiRequestContext, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent},
    grpc::{GatewayCommand, utils::parse_client_ip_agent},
    mail::templates::mfa_code_mail,
};

const CLIENT_SESSION_TIMEOUT: u64 = 60 * 5; // 5 minutes

// How much time the user has to approve remote MFA with mobile device
const REMOTE_AUTH_TIMEOUT: Duration = Duration::from_mins(1);

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

pub enum SessionDisconnectReason {
    /// Closed because a new authorization is creating a replacement session.
    Superseded,
    /// Closed for any other reason (normal teardown).
    Disconnected,
}

pub struct ClientMfaServer {
    pub(crate) pool: PgPool,
    gateway_tx: Sender<GatewayCommand>,
    pub(crate) sessions: Arc<RwLock<HashMap<String, ClientLoginSession>>>,
    remote_mfa_responses: Arc<RwLock<HashMap<String, oneshot::Sender<String>>>>,
    bidi_event_tx: UnboundedSender<BidiStreamEvent>,
}

impl ClientMfaServer {
    fn build_authorized_gateway_network_info(
        network_device: WireguardNetworkDevice,
        preshared_key: String,
    ) -> DeviceNetworkInfo {
        DeviceNetworkInfo::from_authorized_vpn_session(
            network_device.wireguard_network_id,
            network_device.wireguard_ips,
            preshared_key,
        )
    }

    #[must_use]
    pub fn new(
        pool: PgPool,
        gateway_tx: Sender<GatewayCommand>,
        bidi_event_tx: UnboundedSender<BidiStreamEvent>,
        remote_mfa_responses: Arc<RwLock<HashMap<String, oneshot::Sender<String>>>>,
        sessions: Arc<RwLock<HashMap<String, ClientLoginSession>>>,
    ) -> Self {
        Self {
            pool,
            gateway_tx,
            sessions,
            remote_mfa_responses,
            bidi_event_tx,
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

    /// Emit given event to the channel.
    pub(crate) fn emit_event(&self, event: BidiStreamEvent) -> Result<(), ClientMfaServerError> {
        Ok(self.bidi_event_tx.send(event)?)
    }

    /// Allows Edge to verify if token is valid and active.
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
        info: Option<proxy::DeviceInfo>,
    ) -> Result<ClientMfaStartOutcome, Status> {
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
        // `password_management_disabled` is irrelevant here (internal access validation,
        // not an API response), so the OIDC flag is not loaded.
        let user_info = UserInfo::from_user(&self.pool, user.clone(), false)
            .await
            .map_err(|_| {
                error!("Failed to fetch user info for {}", user.username);
                Status::internal("unexpected error")
            })?;

        // validate user is allowed to connect to a given location
        Self::validate_location_access(&self.pool, &location, &device, &user_info).await?;

        // Evaluate postures if necessary.
        let has_postures = location.has_postures(&self.pool).await.map_err(|err| {
            error!(
                "Failed to fetch postures for location {}({}): {err}",
                location.name, location.id
            );
            Status::internal("unexpected error")
        })?;
        if has_postures {
            let posture_result = match validate_posture(
                &self.pool,
                location.id,
                &request.pubkey,
                request.posture_data.as_ref(),
            )
            .await
            {
                Ok(result) => result,
                Err(PostureCheckError::NoActiveEnterpriseLicense) => {
                    debug!("No active license - skipping posture check for location {location}");
                    PostureResult::Pass
                }
                Err(PostureCheckError::DbError(e)) => {
                    error!("DB error during posture validation: {e}");
                    return Err(Status::internal("unexpected error"));
                }
            };

            let (ip, _user_agent) = parse_client_ip_agent(&info).map_err(Status::internal)?;
            let context =
                BidiRequestContext::new(user.id, user.username.clone(), ip, device.name.clone());

            match posture_result {
                PostureResult::Fail(reasons) => {
                    let failed_checks = reasons.iter().map(ToString::to_string).collect::<Vec<_>>();
                    if let Err(err) = self.emit_event(BidiStreamEvent {
                        context,
                        event: BidiStreamEventType::DesktopClientMfa(Box::new(
                            DesktopClientMfaEvent::PostureCheckFailed {
                                device: device.clone(),
                                location: location.clone(),
                                device_posture_data: request.posture_data.clone(),
                                failed_checks: failed_checks.clone(),
                            },
                        )),
                    }) {
                        error!("Failed to emit DevicePostureCheckFailed event: {err}");
                    }
                    self.revoke_rejected_posture_sessions(&location, &user, &device, ip)
                        .await?;
                    return Ok(ClientMfaStartOutcome::Rejected { failed_checks });
                }
                PostureResult::Pass => {
                    if let Err(err) = self.emit_event(BidiStreamEvent {
                        context,
                        event: BidiStreamEventType::DesktopClientMfa(Box::new(
                            DesktopClientMfaEvent::PostureCheckPassed {
                                device: device.clone(),
                                location: location.clone(),
                                device_posture_data: request.posture_data.clone(),
                            },
                        )),
                    }) {
                        error!("Failed to emit DevicePostureCheckPassed event: {err}");
                    }
                }
            }
        }

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

        // Derive the legacy single-factor mode for this location. `None` means the location's
        // flow configuration cannot be expressed as a legacy mode (multi-flow, multi-step, or a
        // subset of the internal method set), so no current client can enforce it. Fail closed
        // rather than fall back to a mode: `mfa_enabled` is a stored column now, so it no longer
        // implies that a legacy mode is derivable.
        let Some(location_mfa_mode) = MfaFlow::derive_legacy_mode(&self.pool, request.location_id)
            .await
            .map_err(|err| {
                error!("Failed to derive legacy MFA mode: {err}");
                Status::internal("unexpected error")
            })?
        else {
            error!(
                "Location {location} has an MFA flow configuration that cannot be enforced by \
                this client"
            );
            return Err(Status::failed_precondition(
                "location MFA configuration is not supported by this client",
            ));
        };

        // check if selected MFA method matches location settings
        match (&location_mfa_mode, selected_method) {
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
                    {location}"
                );

                return Err(Status::invalid_argument(
                    "selected MFA method is not supported by location",
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
                        "Select MFA method is not available for the device.",
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
                        "selected MFA method is not available",
                    ));
                }
            }
            MfaMethod::Totp => {
                if !user.totp_enabled {
                    error!("TOTP not enabled for user {}", user.username);
                    return Err(Status::invalid_argument(
                        "selected MFA method is not available",
                    ));
                }
            }
            MfaMethod::Email => {
                if !user.email_mfa_enabled {
                    error!("Email MFA not enabled for user {}", user.username);
                    return Err(Status::invalid_argument(
                        "selected MFA method is not available",
                    ));
                }
                // Generate the code and send it via email.
                let code = user.generate_email_mfa_code().map_err(|err| {
                    error!("Failed to generate email MFA code: {err}");
                    Status::internal("MFA code")
                })?;
                let mut transaction = self.pool.begin().await.map_err(|err| {
                    error!("Database error: {err}");
                    Status::internal("database error")
                })?;
                mfa_code_mail(
                    &user.email,
                    &mut transaction,
                    &user.first_name,
                    &code,
                    None,
                    true,
                )
                .await
                .map_err(|err| {
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
                        "selected MFA method is not available",
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
                        "selected MFA method is not available",
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
                    let challenge = BiometricChallenge::new_with_owner(&mobile_auth.pub_key)
                        .map_err(|e| {
                            error!(
                                "Start biometric MFA failed. Challenge creation failed. Reason: {e}"
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

        Ok(ClientMfaStartOutcome::Approved(ClientMfaStartResponse {
            token,
            challenge: response_challenge,
        }))
    }

    /// Checks whether the user and device are allowed to access a location.
    async fn validate_location_access(
        pool: &PgPool,
        location: &WireguardNetwork<Id>,
        device: &Device<Id>,
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
        // If not all groups are allowed, check if user belongs to one of the allowed groups.
        if !location.allow_all_groups
            && !allowed_groups
                .iter()
                .any(|allowed_group| user_info.groups.contains(allowed_group))
        {
            error!(
                "User {} is not allowed to connect to location {location} because he/she doesn't \
                belong to any of the allowed groups. User groups: {:?}, allowed groups: \
                {allowed_groups:?}",
                user_info.username, user_info.groups
            );
            return Err(Status::unauthenticated("unauthorized"));
        }

        let assignment = WireguardNetworkDevice::find(&mut *conn, device.id, location.id)
            .await
            .map_err(|err| {
                error!(
                    "Failed to validate assignment for device {device} in location {location}: \
                    {err}"
                );
                Status::internal("unexpected error")
            })?;
        if assignment.is_none() {
            error!("Device {device} is not assigned to location {location}");
            return Err(Status::permission_denied(
                "device is not assigned to location",
            ));
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
        let context =
            BidiRequestContext::new(user.id, user.username.clone(), ip, format!("{device}"));

        // name of the device used to approve a mobile approve login; populated below
        let mut mobile_auth_device_name: Option<String> = None;

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
                // record the approving device's name for the success activity log event
                mobile_auth_device_name =
                    BiometricAuth::find_device(&self.pool, user.id, &auth_device_pub_key)
                        .await
                        .map_err(|_| Status::internal("unexpected error"))?
                        .map(|auth_device| auth_device.name);
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
                                    message: "Signed challenge rejected".to_owned(),
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
                                    message: "Signed challenge rejected".to_owned(),
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
                                message: "TOTP code not provided in request".to_owned(),
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
                                message: "invalid TOTP code".to_owned(),
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
                                message: "email MFA code not provided in request".to_owned(),
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
                                message: "invalid email MFA code".to_owned(),
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
                                    .to_owned(),
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
        let Ok(Some(network_device)) =
            WireguardNetworkDevice::find(&mut *transaction, device.id, location.id).await
        else {
            error!("Failed to fetch network config for device {device} and location {location}");
            return Err(Status::internal("unexpected error"));
        };

        // generate PSK
        let key = WireguardNetwork::genkey();

        // create new VPN client session
        let vpn_client_session = self
            .create_new_session(
                &mut transaction,
                &location,
                &user,
                &device,
                Some(method.into()),
                key.public.clone(),
            )
            .await
            .map_err(|err| {
                error!("Failed to create new VPN client session for device {device} in location {location}: {err}");
                Status::internal("unexpected error")
            })?;
        debug!("Created new VPN client session: {vpn_client_session:?}");

        let gateway_network_info =
            Self::build_authorized_gateway_network_info(network_device, key.public.clone());

        // send gateway event
        debug!("Sending `peer_create` message to gateway");
        let event =
            GatewayCommand::VpnSessionAuthorized(location.id, device.clone(), gateway_network_info);
        self.gateway_tx.send(event).map_err(|err| {
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
                DesktopClientMfaEvent::Success {
                    location: location.clone(),
                    device: device.clone(),
                    method,
                    mobile_auth_device_name,
                },
            )),
        })?;

        let response = ClientMfaFinishResponse {
            preshared_key: key.public.clone(),
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
        if let Some(tx) = self
            .remote_mfa_responses
            .write()
            .expect("Failed to write-lock ClientMfaServer::remote_mfa_responses")
            .remove(&request.token)
        {
            let _ = tx.send(key.public.clone());
        }

        Ok(response)
    }

    /// Handles a `PostureCheck` request from the proxy bidi stream.
    ///
    /// Validates the posture data, and on success creates a new `VpnClientSession`
    /// with a generated preshared key. Returns a typed outcome so the caller can
    /// map it to the appropriate `CoreResponse` payload without needing to know about
    /// session internals.
    ///
    /// A location with no postures assigned is approved with an *empty* preshared key and no
    /// session, since its peers are handed to the gateway without one.
    pub async fn handle_posture_check(
        &mut self,
        request: DevicePostureCheckRequest,
        info: Option<proxy::DeviceInfo>,
    ) -> Result<PostureCheckOutcome, Status> {
        debug!(
            "Handling posture check for device pubkey={} location_id={}",
            request.pubkey, request.location_id
        );

        // Authenticate the caller before touching anything else.
        // Validated first so that an unauthenticated caller cannot use the error codes below to
        // probe which locations exist or which public keys are enrolled.
        let Some(token) = request.token.as_deref().filter(|token| !token.is_empty()) else {
            error!(
                "Posture check: missing polling token for pubkey {}",
                request.pubkey
            );
            return Err(Status::unauthenticated("missing token"));
        };
        let polling_token = PollingToken::find(&self.pool, token)
            .await
            .map_err(|err| {
                error!("Posture check: failed to look up polling token: {err}");
                Status::internal("unexpected error")
            })?
            .ok_or_else(|| {
                error!(
                    "Posture check: unknown polling token for claimed pubkey {}",
                    request.pubkey
                );
                Status::unauthenticated("invalid token")
            })?;

        // Look up location, device, and user.
        let Ok(Some(location)) =
            WireguardNetwork::find_by_id(&self.pool, request.location_id).await
        else {
            error!("Posture check: location {} not found", request.location_id);
            return Err(Status::invalid_argument("location not found"));
        };

        if location.mfa_enabled() {
            error!(
                "Posture check: location {location} has MFA enabled, posture-only sessions are not allowed"
            );
            return Err(Status::invalid_argument("location has MFA enabled"));
        }

        let Ok(Some(device)) = Device::find_by_pubkey(&self.pool, &request.pubkey).await else {
            error!(
                "Posture check: device with pubkey {} not found",
                request.pubkey
            );
            return Err(Status::invalid_argument("device not found"));
        };

        // Make sure caller owns the device.
        if polling_token.device_id != device.id {
            error!(
                "Posture check: polling token belongs to device {} but request claims pubkey {} \
                (device {})",
                polling_token.device_id, request.pubkey, device.id
            );
            return Err(Status::unauthenticated("token does not match device"));
        }

        let Ok(Some(user)) = User::find_by_id(&self.pool, device.user_id).await else {
            error!("Posture check: user {} not found", device.user_id);
            return Err(Status::internal("user not found"));
        };

        // Ensure user is active
        if !user.is_active {
            error!("Posture check: user {} is inactive", device.user_id);
            return Err(Status::invalid_argument("user is inactive"));
        }

        // Validate that the user is allowed to access this location.
        // `password_management_disabled` is irrelevant here (internal access validation,
        // not an API response), so the OIDC flag is not loaded.
        let user_info = UserInfo::from_user(&self.pool, user.clone(), false)
            .await
            .map_err(|_| {
                error!(
                    "Posture check: failed to fetch user info for {}",
                    user.username
                );
                Status::internal("unexpected error")
            })?;
        Self::validate_location_access(&self.pool, &location, &device, &user_info).await?;

        // If location has no postures assigned, approve the posture check returning empty string as PSK.
        // This way the client can recover on it's own if the admin unassigns PCs from a location and the client
        // didn't get the config yet. Matters especially for service locations where the client UI may not be
        // running and therefore config is not being polled.
        if !location.has_postures(&self.pool).await.map_err(|err| {
            error!("Posture check: failed to fetch postures for location {location}: {err}");
            Status::internal("unexpected error")
        })? {
            info!(
                "Posture check: location {location} has no postures assigned, approving device {} \
                with an empty preshared key without creating a session",
                device.wireguard_pubkey
            );
            return Ok(PostureCheckOutcome::Approved {
                preshared_key: String::new(),
            });
        }

        let posture_result = match validate_posture(
            &self.pool,
            location.id,
            &device.wireguard_pubkey,
            request.device_posture_data.as_ref(),
        )
        .await
        {
            Ok(result) => result,
            Err(PostureCheckError::NoActiveEnterpriseLicense) => {
                debug!("No active license - skipping posture check for location {location}");
                PostureResult::Pass
            }
            Err(PostureCheckError::DbError(e)) => {
                error!("DB error during posture validation: {e}");
                return Err(Status::internal("unexpected error"));
            }
        };

        let (ip, _user_agent) = parse_client_ip_agent(&info).map_err(Status::internal)?;
        let context =
            BidiRequestContext::new(user.id, user.username.clone(), ip, device.name.clone());

        // Posture check failed - return payload with reasons
        if let PostureResult::Fail(reasons) = posture_result {
            let failed_checks = reasons.iter().map(ToString::to_string).collect::<Vec<_>>();
            if let Err(err) = self.emit_event(BidiStreamEvent {
                context,
                event: BidiStreamEventType::DesktopClientMfa(Box::new(
                    DesktopClientMfaEvent::PostureCheckFailed {
                        device: device.clone(),
                        location: location.clone(),
                        device_posture_data: request.device_posture_data.clone(),
                        failed_checks: failed_checks.clone(),
                    },
                )),
            }) {
                error!("Failed to emit DevicePostureCheckFailed event: {err}");
            }

            self.revoke_rejected_posture_sessions(&location, &user, &device, ip)
                .await?;

            return Ok(PostureCheckOutcome::Rejected { failed_checks });
        }

        if let Err(err) = self.emit_event(BidiStreamEvent {
            context,
            event: BidiStreamEventType::DesktopClientMfa(Box::new(
                DesktopClientMfaEvent::PostureCheckPassed {
                    device: device.clone(),
                    location: location.clone(),
                    device_posture_data: request.device_posture_data.clone(),
                },
            )),
        }) {
            error!("Failed to emit DevicePostureCheckPassed event: {err}");
        }

        // Posture check succeeded - create a vpn session
        let key = WireguardNetwork::genkey();

        let mut transaction = self.pool.begin().await.map_err(|err| {
            error!("Failed to begin transaction for posture session: {err}");
            Status::internal("unexpected error")
        })?;

        let Ok(Some(network_device)) =
            WireguardNetworkDevice::find(&mut *transaction, device.id, location.id).await
        else {
            error!(
                "Posture check: failed to fetch network config for device {device} and location {location}"
            );
            return Err(Status::internal("unexpected error"));
        };

        let gateway_network_info =
            Self::build_authorized_gateway_network_info(network_device, key.public.clone());

        self.create_new_session(
            &mut transaction,
            &location,
            &user,
            &device,
            None, // posture-only session has no MFA method
            key.public.clone(),
        )
        .await?;

        transaction.commit().await.map_err(|err| {
            error!("Failed to commit transaction for posture session: {err}");
            Status::internal("unexpected error")
        })?;

        let event =
            GatewayCommand::VpnSessionAuthorized(location.id, device.clone(), gateway_network_info);
        self.gateway_tx.send(event).map_err(|err| {
            error!("Error sending WireGuard event: {err}");
            Status::internal("unexpected error")
        })?;

        info!(
            "Posture check passed for device {} (user {}) in location {}. Session created.",
            device, user.username, location
        );

        Ok(PostureCheckOutcome::Approved {
            preshared_key: key.public,
        })
    }

    /// Revokes sessions after a definitive posture rejection and publishes resulting events.
    async fn revoke_rejected_posture_sessions(
        &self,
        location: &WireguardNetwork<Id>,
        user: &User<Id>,
        device: &Device<Id>,
        ip: IpAddr,
    ) -> Result<(), Status> {
        let mut transaction = self.pool.begin().await.map_err(|err| {
            error!("Failed to begin transaction for posture session rejection: {err}");
            Status::internal("unexpected error")
        })?;
        let disconnect_events = self
            .revoke_active_posture_sessions(&mut transaction, location, user, device, ip)
            .await?;
        transaction.commit().await.map_err(|err| {
            error!("Failed to commit rejected posture session cleanup: {err}");
            Status::internal("unexpected error")
        })?;

        let event = GatewayCommand::VpnSessionDeauthorized(location.id, device.clone());
        if let Err(err) = self.gateway_tx.send(event) {
            error!("Error sending WireGuard event: {err}");
        }
        for event in disconnect_events {
            if let Err(err) = self.emit_event(event) {
                error!("Failed to emit VPN session disconnect event: {err}");
            }
        }

        Ok(())
    }

    /// Marks active posture sessions disconnected and returns their audit events.
    async fn revoke_active_posture_sessions(
        &self,
        conn: &mut PgConnection,
        location: &WireguardNetwork<Id>,
        user: &User<Id>,
        device: &Device<Id>,
        ip: IpAddr,
    ) -> Result<Vec<BidiStreamEvent>, Status> {
        let active_sessions = VpnClientSession::get_all_active_device_sessions_in_location(
            &mut *conn,
            location.id,
            device.id,
        )
        .await
        .map_err(|err| {
            error!(
                "Failed to fetch active VPN sessions for device {device} in location {location}: {err}"
            );
            Status::internal("unexpected error")
        })?;
        if !active_sessions.is_empty() {
            info!(
                "Posture check rejected device {device} in location {location}. Disconnecting {} active sessions",
                active_sessions.len()
            );
        }

        let mut events = Vec::new();
        for mut session in active_sessions {
            let is_connected = session.state == VpnClientSessionState::Connected;
            let is_mfa_session = session.mfa_method.is_some();
            let disconnect_timestamp = Utc::now().naive_utc();
            session.disconnected_at = Some(disconnect_timestamp);
            session.state = VpnClientSessionState::Disconnected;
            session.save(&mut *conn).await.map_err(|err| {
                error!("Failed to revoke rejected posture session {session:?}: {err}");
                Status::internal("unexpected error")
            })?;

            if is_connected {
                events.push(BidiStreamEvent {
                    context: BidiRequestContext {
                        timestamp: disconnect_timestamp,
                        user_id: user.id,
                        username: user.username.clone(),
                        ip: Some(ip),
                        device_name: format!("{device}"),
                    },
                    event: BidiStreamEventType::DesktopClientMfa(Box::new(
                        DesktopClientMfaEvent::Disconnected {
                            location: location.clone(),
                            device: device.clone(),
                            is_mfa_session,
                        },
                    )),
                });
            }
        }

        Ok(events)
    }

    /// Helper used to close all existing active sessions while creating a new MFA session
    /// and send relevant gateway updates
    async fn create_new_session(
        &self,
        conn: &mut PgConnection,
        location: &WireguardNetwork<Id>,
        user: &User<Id>,
        device: &Device<Id>,
        mfa_method: Option<VpnClientMfaMethod>,
        preshared_key: String,
    ) -> Result<VpnClientSession<Id>, Status> {
        debug!(
            "Creating new VPN session for device {device} of user {user} in location {location}."
        );

        // find all active sessions for a given device and location
        let active_sessions = VpnClientSession::get_all_active_device_sessions_in_location(
            &mut *conn,
            location.id,
            device.id,
        )
        .await
        .map_err(|err| {
            error!(
                "Failed to fetch active VPN sessions for device {device} in location {location}: {err}"
            );
            Status::internal("unexpected error")
        })?;
        if !active_sessions.is_empty() {
            info!(
                "Found {} active sessions for device {device} in location {location}. Disconnecting them before creating a new MFA session",
                active_sessions.len()
            );
        }

        // disconnect all active sessions
        for session in active_sessions {
            debug!("Disconnecting previous active MFA VPN session {session:?}.");
            self.disconnect_session(
                &mut *conn,
                session,
                location,
                user,
                device,
                SessionDisconnectReason::Superseded,
            )
            .await?;
        }

        // create new MFA session
        let mut session = VpnClientSession::new(location.id, user.id, device.id, None, mfa_method);
        session.preshared_key = Some(preshared_key);
        session.save(conn).await.map_err(|err| {
            error!("Failed to create new VPN client session for device {device} in location {location}: {err}");
            Status::internal("unexpected error")
        })
    }

    /// Update session state as disconnected and send relevant gateway update
    async fn disconnect_session(
        &self,
        conn: &mut PgConnection,
        mut session: VpnClientSession<Id>,
        location: &WireguardNetwork<Id>,
        user: &User<Id>,
        device: &Device<Id>,
        reason: SessionDisconnectReason,
    ) -> Result<(), Status> {
        let is_connected = session.state == VpnClientSessionState::Connected;
        let is_mfa_session = session.mfa_method.is_some();
        let requires_gateway_update = is_mfa_session
            || location.has_postures(&mut *conn).await.map_err(|err| {
                error!("Failed to fetch postures for location {location}: {err}");
                Status::internal("unexpected error")
            })?;

        // update session state in DB
        let disconnect_timestamp = Utc::now().naive_utc();
        session.disconnected_at = Some(disconnect_timestamp);
        session.state = VpnClientSessionState::Disconnected;
        session.save(&mut *conn).await.map_err(|err| {
            error!("Failed to update VPN session {session:?}: {err}");
            Status::internal("unexpected error")
        })?;

        // gateway update is only needed to remove peers that were authorized at runtime - MFA and posture-check sessions
        // this is needed to remove peers for both Connected and New sessions
        if requires_gateway_update {
            let gateway_event = GatewayCommand::VpnSessionDeauthorized(location.id, device.clone());
            self.gateway_tx.send(gateway_event).map_err(|err| {
                error!("Error sending WireGuard event: {err}");
                Status::internal("unexpected error")
            })?;
        }

        // only emit disconnect events if a session has actually been connected
        if is_connected {
            let context = BidiRequestContext {
                timestamp: disconnect_timestamp,
                user_id: user.id,
                username: user.username.clone(),
                ip: None,
                device_name: format!("{device}"),
            };
            let event = match reason {
                SessionDisconnectReason::Superseded => DesktopClientMfaEvent::SessionSuperseded {
                    location: location.clone(),
                    device: device.clone(),
                    is_mfa_session,
                },
                SessionDisconnectReason::Disconnected => DesktopClientMfaEvent::Disconnected {
                    location: location.clone(),
                    device: device.clone(),
                    is_mfa_session,
                },
            };
            self.emit_event(BidiStreamEvent {
                context,
                event: BidiStreamEventType::DesktopClientMfa(Box::new(event)),
            })
            .map_err(Status::from)?;
        }

        Ok(())
    }
}

/// Result of a [`ClientMfaServer::handle_posture_check`] call.
pub enum PostureCheckOutcome {
    /// Posture evaluation passed; the contained key must be returned to the client.
    Approved { preshared_key: String },
    /// Posture evaluation failed; the contained list describes which checks failed.
    Rejected { failed_checks: Vec<String> },
}

/// Result of a [`ClientMfaServer::start_client_mfa_login`] call.
/// Adds posture check outcome info.
pub enum ClientMfaStartOutcome {
    /// Posture evaluation succeeded or was unnecessary.
    Approved(ClientMfaStartResponse),
    /// Posture evaluation failed; the contained list describes which checks failed.
    Rejected { failed_checks: Vec<String> },
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        net::{IpAddr, Ipv4Addr},
        sync::{Arc, RwLock},
    };

    use chrono::Utc;
    use defguard_common::db::{
        Id,
        models::{
            Device, DeviceType, User, WireguardNetwork,
            device::WireguardNetworkDevice,
            polling_token::PollingToken,
            settings::initialize_current_settings,
            vpn_client_session::{VpnClientMfaMethod, VpnClientSession, VpnClientSessionState},
            wireguard::ServiceLocationMode,
        },
        setup_pool,
    };
    use defguard_proto::{
        client_types::{ClientMfaStartRequest, MfaMethod},
        enterprise::posture::{
            BoolCheck, DevicePostureCheckRequest, DevicePostureData, bool_check,
        },
        proxy::DeviceInfo,
    };
    use ipnetwork::IpNetwork;
    use sqlx::{
        PgPool,
        postgres::{PgConnectOptions, PgPoolOptions},
    };
    use tokio::sync::{broadcast, mpsc, oneshot};
    use tonic::Code;

    use super::{ClientLoginSession, ClientMfaServer};
    use crate::{
        enterprise::{
            db::models::device_posture::{
                DevicePosture, DevicePostureLocation, DevicePostureOsRule, OsType,
            },
            license::{License, LicenseTier, SupportType, set_cached_license},
            limits::{Counts, set_counts},
        },
        events::{BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent},
        grpc::{GatewayCommand, proto::enterprise::license::LicenseLimits},
    };

    const REPLACEMENT_MFA_PRESHARED_KEY: &str = "replacement-mfa-psk";
    const NEW_MFA_PRESHARED_KEY: &str = "new-psk";
    const DEVICE_INFO_IP: &str = "10.0.0.7";

    /// The `DeviceInfo` the proxy attaches to every bidi request; audit events are built from it.
    fn device_info() -> Option<DeviceInfo> {
        Some(DeviceInfo {
            ip_address: DEVICE_INFO_IP.to_owned(),
            user_agent: Some("defguard-client/1.6.0".to_owned()),
            ..Default::default()
        })
    }

    #[sqlx::test]
    async fn test_posture_check_success_emits_vpn_session_authorized_event(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        set_enterprise_license();
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");
        let location = create_non_mfa_location(&pool).await;
        save_linux_posture_policy(&pool, location.id).await;
        let user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, device.id).await;
        let token = create_polling_token(&pool, device.id).await;
        let (mut server, _event_rx, mut gateway_rx) = make_server(pool.clone());

        let outcome = server
            .handle_posture_check(
                DevicePostureCheckRequest {
                    location_id: location.id,
                    pubkey: device.wireguard_pubkey.clone(),
                    device_posture_data: Some(passing_linux_posture_data()),
                    token: Some(token.clone()),
                },
                device_info(),
            )
            .await
            .expect("posture check should pass");
        let preshared_key = match outcome {
            super::PostureCheckOutcome::Approved { preshared_key } => preshared_key,
            super::PostureCheckOutcome::Rejected { failed_checks } => {
                panic!("posture check unexpectedly failed: {failed_checks:?}")
            }
        };

        match gateway_rx
            .try_recv()
            .expect("expected VPN authorization gateway event")
        {
            GatewayCommand::VpnSessionAuthorized(location_id, authorized_device, network_info) => {
                assert_eq!(location_id, location.id);
                assert_eq!(authorized_device.id, device.id);
                assert_eq!(network_info.network_id, location.id);
                assert_eq!(
                    network_info.preshared_key.as_deref(),
                    Some(preshared_key.as_str())
                );
                assert!(network_info.is_authorized);
            }
            other => panic!("unexpected gateway event: {other:?}"),
        }

        let active_sessions = VpnClientSession::get_all_active_device_sessions_in_location(
            &pool,
            location.id,
            device.id,
        )
        .await
        .expect("failed to fetch active sessions");
        assert_eq!(active_sessions.len(), 1);
        assert_eq!(
            active_sessions[0].preshared_key.as_deref(),
            Some(preshared_key.as_str())
        );
    }

    #[sqlx::test]
    async fn test_replacing_posture_session_emits_vpn_session_deauthorized_event(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        set_enterprise_license();
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");
        let location = create_non_mfa_location(&pool).await;
        save_linux_posture_policy(&pool, location.id).await;
        let user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, device.id).await;
        let mut old_session = VpnClientSession::new(
            location.id,
            user.id,
            device.id,
            Some(Utc::now().naive_utc()),
            None,
        );
        old_session.preshared_key = Some("old-posture-psk".to_owned());
        old_session.state = VpnClientSessionState::Connected;
        let old_session = old_session
            .save(&pool)
            .await
            .expect("failed to create previous posture session");
        let token = create_polling_token(&pool, device.id).await;
        let (mut server, mut event_rx, mut gateway_rx) = make_server(pool.clone());

        server
            .handle_posture_check(
                DevicePostureCheckRequest {
                    location_id: location.id,
                    pubkey: device.wireguard_pubkey.clone(),
                    device_posture_data: Some(passing_linux_posture_data()),
                    token: Some(token.clone()),
                },
                device_info(),
            )
            .await
            .expect("replacement posture check should pass");

        match gateway_rx
            .try_recv()
            .expect("expected VPN deauthorization gateway event for replaced posture session")
        {
            GatewayCommand::VpnSessionDeauthorized(location_id, disconnected_device) => {
                assert_eq!(location_id, location.id);
                assert_eq!(disconnected_device.id, device.id);
            }
            other => panic!("unexpected gateway event: {other:?}"),
        }
        match gateway_rx
            .try_recv()
            .expect("expected VPN authorization gateway event for replacement posture session")
        {
            GatewayCommand::VpnSessionAuthorized(location_id, authorized_device, network_info) => {
                assert_eq!(location_id, location.id);
                assert_eq!(authorized_device.id, device.id);
                assert!(network_info.preshared_key.is_some());
            }
            other => panic!("unexpected gateway event: {other:?}"),
        }

        // the passing posture evaluation is audited first
        let event = event_rx
            .try_recv()
            .expect("expected posture check passed audit event");
        match event.event {
            BidiStreamEventType::DesktopClientMfa(event) => match *event {
                DesktopClientMfaEvent::PostureCheckPassed { .. } => {}
                other => panic!("unexpected bidi event: {other:?}"),
            },
            other => panic!("unexpected bidi stream event type: {other:?}"),
        }

        // replacing a connected posture-only session emits the unified session
        // superseded audit event, flagged as a non-MFA session
        let event = event_rx
            .try_recv()
            .expect("expected session replaced audit event for replaced posture session");
        match event.event {
            BidiStreamEventType::DesktopClientMfa(event) => match *event {
                DesktopClientMfaEvent::SessionSuperseded {
                    location: event_location,
                    device: event_device,
                    is_mfa_session,
                } => {
                    assert_eq!(event_location.id, location.id);
                    assert_eq!(event_device.id, device.id);
                    assert!(!is_mfa_session);
                }
                other => panic!("unexpected bidi event: {other:?}"),
            },
            other => panic!("unexpected bidi stream event type: {other:?}"),
        }

        let old_session = VpnClientSession::find_by_id(&pool, old_session.id)
            .await
            .expect("failed to reload old posture session")
            .expect("expected old posture session");
        assert_eq!(old_session.state, VpnClientSessionState::Disconnected);
    }

    /// A caller with no token must be refused. Without this, knowing a device's public key is
    /// enough to mint a preshared key for it.
    #[sqlx::test]
    async fn test_posture_check_requires_a_token(_: PgPoolOptions, options: PgConnectOptions) {
        set_enterprise_license();
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");
        let location = create_non_mfa_location(&pool).await;
        save_linux_posture_policy(&pool, location.id).await;
        let user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, device.id).await;
        let (mut server, _, mut gateway_rx) = make_server(pool.clone());

        for token in [None, Some(String::new())] {
            let err = server
                .handle_posture_check(
                    DevicePostureCheckRequest {
                        location_id: location.id,
                        pubkey: device.wireguard_pubkey.clone(),
                        device_posture_data: Some(passing_linux_posture_data()),
                        token,
                    },
                    device_info(),
                )
                .await;
            let err = match err {
                Ok(_) => panic!("posture check without a token must be refused"),
                Err(err) => err,
            };
            assert_eq!(err.code(), Code::Unauthenticated);
        }

        // No session may be created and the gateway must not be touched.
        assert!(
            VpnClientSession::get_all_active_device_sessions_in_location(
                &pool,
                location.id,
                device.id
            )
            .await
            .expect("failed to query sessions")
            .is_empty()
        );
        assert!(gateway_rx.try_recv().is_err());
    }

    /// An unknown token must be refused, so tokens cannot be guessed or replayed after rotation.
    #[sqlx::test]
    async fn test_posture_check_rejects_unknown_token(_: PgPoolOptions, options: PgConnectOptions) {
        set_enterprise_license();
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");
        let location = create_non_mfa_location(&pool).await;
        save_linux_posture_policy(&pool, location.id).await;
        let user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, device.id).await;
        let (mut server, _, _) = make_server(pool);

        let err = server
            .handle_posture_check(
                DevicePostureCheckRequest {
                    location_id: location.id,
                    pubkey: device.wireguard_pubkey.clone(),
                    device_posture_data: Some(passing_linux_posture_data()),
                    token: Some("not-a-real-token".to_owned()),
                },
                device_info(),
            )
            .await;
        let err = match err {
            Ok(_) => panic!("posture check with an unknown token must be refused"),
            Err(err) => err,
        };

        assert_eq!(err.code(), Code::Unauthenticated);
    }

    /// Regression test for the session-hijack denial of service: holding a valid token for *one*
    /// device must not allow authorizing — and thereby superseding the live session of — another.
    #[sqlx::test]
    async fn test_posture_check_rejects_token_belonging_to_another_device(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        set_enterprise_license();
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");
        let location = create_non_mfa_location(&pool).await;
        save_linux_posture_policy(&pool, location.id).await;
        let user = create_user(&pool).await;

        let victim = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, victim.id).await;

        // The attacker is a legitimately enrolled device with a token of its own.
        let attacker = Device::new(
            "attacker-device".to_owned(),
            "attacker-pubkey".to_owned(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .expect("failed to create attacker device");
        let attacker_token = create_polling_token(&pool, attacker.id).await;

        // The victim holds a live session.
        let mut victim_session = VpnClientSession::new(
            location.id,
            user.id,
            victim.id,
            Some(Utc::now().naive_utc()),
            None,
        );
        victim_session.preshared_key = Some("victim-psk".to_owned());
        victim_session.state = VpnClientSessionState::Connected;
        let victim_session = victim_session
            .save(&pool)
            .await
            .expect("failed to create victim session");

        let (mut server, _, mut gateway_rx) = make_server(pool.clone());

        // Attacker presents its own valid token but claims the victim's public key.
        let err = server
            .handle_posture_check(
                DevicePostureCheckRequest {
                    location_id: location.id,
                    pubkey: victim.wireguard_pubkey.clone(),
                    device_posture_data: Some(passing_linux_posture_data()),
                    token: Some(attacker_token),
                },
                device_info(),
            )
            .await;
        let err = match err {
            Ok(_) => panic!("a token from another device must not authorize this one"),
            Err(err) => err,
        };
        assert_eq!(err.code(), Code::Unauthenticated);

        // The victim's session must survive untouched, and the gateway must see nothing.
        let victim_session = VpnClientSession::find_by_id(&pool, victim_session.id)
            .await
            .expect("failed to reload victim session")
            .expect("victim session should still exist");
        assert_eq!(victim_session.state, VpnClientSessionState::Connected);
        assert_eq!(
            victim_session.preshared_key.as_deref(),
            Some("victim-psk"),
            "the victim's preshared key must not have been rotated"
        );
        assert!(
            gateway_rx.try_recv().is_err(),
            "no peer delete or re-create may be sent to the gateway"
        );
    }

    #[sqlx::test]
    async fn test_posture_check_rejects_mfa_enabled_location(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let location = create_mfa_location(&pool).await;
        // A valid token is needed to get past authentication and reach the check under test.
        let user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;
        let token = create_polling_token(&pool, device.id).await;
        let (mut server, _, _) = make_server(pool);

        let err = match server
            .handle_posture_check(
                DevicePostureCheckRequest {
                    location_id: location.id,
                    pubkey: "irrelevant".to_owned(),
                    device_posture_data: None,
                    token: Some(token),
                },
                device_info(),
            )
            .await
        {
            Ok(_) => panic!("MFA-enabled location should reject posture-only flow"),
            Err(err) => err,
        };

        assert_eq!(err.code(), Code::InvalidArgument);
    }

    /// A location with no postures assigned hands its peers to the gateway without a preshared
    /// key, so the only answer that lets a client connect is an empty one. Approving instead of
    /// erroring is what allows a service location whose cached config still demands a posture check
    /// to recover after an admin unassigns the last posture.
    #[sqlx::test]
    async fn test_posture_check_without_postures_approves_with_empty_preshared_key(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        set_enterprise_license();
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");
        let location = create_non_mfa_location(&pool).await;
        let user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, device.id).await;
        let token = create_polling_token(&pool, device.id).await;
        let (mut server, _event_rx, mut gateway_rx) = make_server(pool.clone());

        let outcome = server
            .handle_posture_check(
                DevicePostureCheckRequest {
                    location_id: location.id,
                    pubkey: device.wireguard_pubkey.clone(),
                    device_posture_data: None,
                    token: Some(token),
                },
                device_info(),
            )
            .await
            .expect("location without postures should be approved");

        match outcome {
            super::PostureCheckOutcome::Approved { preshared_key } => assert!(
                preshared_key.is_empty(),
                "a location without postures must not hand out a preshared key"
            ),
            super::PostureCheckOutcome::Rejected { failed_checks } => {
                panic!("posture check unexpectedly failed: {failed_checks:?}")
            }
        }

        // No session may be created and the gateway must not be touched.
        assert!(
            VpnClientSession::get_all_active_device_sessions_in_location(
                &pool,
                location.id,
                device.id
            )
            .await
            .expect("failed to query sessions")
            .is_empty(),
            "no VPN session may be created when a location has no postures"
        );
        assert!(
            gateway_rx.try_recv().is_err(),
            "no gateway command may be sent when a location has no postures"
        );
    }

    #[sqlx::test]
    async fn test_posture_check_without_postures_rejects_device_not_assigned_to_location(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        set_enterprise_license();
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");
        let location = create_non_mfa_location(&pool).await;
        let user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;
        let token = create_polling_token(&pool, device.id).await;
        let (mut server, mut event_rx, mut gateway_rx) = make_server(pool);

        let status = match server
            .handle_posture_check(
                DevicePostureCheckRequest {
                    location_id: location.id,
                    pubkey: device.wireguard_pubkey,
                    device_posture_data: None,
                    token: Some(token),
                },
                device_info(),
            )
            .await
        {
            Ok(_) => panic!("a device not assigned to the location must not be approved"),
            Err(status) => status,
        };

        assert_eq!(status.code(), Code::PermissionDenied);
        assert_eq!(status.message(), "device is not assigned to location");
        assert!(event_rx.try_recv().is_err());
        assert!(gateway_rx.try_recv().is_err());
    }

    /// The empty-preshared-key approval must not outrank the access checks: deactivating a user has
    /// to stop their devices from getting anything that reads as approval, even on a location with
    /// no postures where the approval grants nothing by itself.
    #[sqlx::test]
    async fn test_posture_check_without_postures_still_rejects_inactive_user(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        set_enterprise_license();
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");
        let location = create_non_mfa_location(&pool).await;
        let mut user = create_user(&pool).await;
        user.is_active = false;
        user.save(&pool).await.expect("failed to deactivate user");
        let device = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, device.id).await;
        let token = create_polling_token(&pool, device.id).await;
        let (mut server, _event_rx, _gateway_rx) = make_server(pool.clone());

        let status = match server
            .handle_posture_check(
                DevicePostureCheckRequest {
                    location_id: location.id,
                    pubkey: device.wireguard_pubkey.clone(),
                    device_posture_data: None,
                    token: Some(token),
                },
                device_info(),
            )
            .await
        {
            Ok(super::PostureCheckOutcome::Approved { .. }) => {
                panic!("an inactive user must not be approved, even without postures")
            }
            Ok(super::PostureCheckOutcome::Rejected { .. }) => {
                panic!("expected an inactive-user error, not a posture rejection")
            }
            Err(status) => status,
        };
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert_eq!(status.message(), "user is inactive");
    }

    /// A passing posture evaluation must be auditable, so an operator can see that a headless
    /// service location connected and why.
    #[sqlx::test]
    async fn test_posture_check_pass_emits_posture_check_passed_event(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        set_enterprise_license();
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");
        let location = create_non_mfa_location(&pool).await;
        save_linux_posture_policy(&pool, location.id).await;
        let user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, device.id).await;
        let token = create_polling_token(&pool, device.id).await;
        let (mut server, mut event_rx, _gateway_rx) = make_server(pool.clone());

        let posture_data = passing_linux_posture_data();
        match server
            .handle_posture_check(
                DevicePostureCheckRequest {
                    location_id: location.id,
                    pubkey: device.wireguard_pubkey.clone(),
                    device_posture_data: Some(posture_data.clone()),
                    token: Some(token),
                },
                device_info(),
            )
            .await
            .expect("posture check should pass")
        {
            super::PostureCheckOutcome::Approved { preshared_key } => {
                assert!(!preshared_key.is_empty());
            }
            super::PostureCheckOutcome::Rejected { failed_checks } => {
                panic!("posture check unexpectedly failed: {failed_checks:?}")
            }
        }

        let event = event_rx
            .try_recv()
            .expect("expected posture check passed audit event");
        match event.event {
            BidiStreamEventType::DesktopClientMfa(event) => match *event {
                DesktopClientMfaEvent::PostureCheckPassed {
                    device: event_device,
                    location: event_location,
                    device_posture_data,
                } => {
                    assert_eq!(event_device.id, device.id);
                    assert_eq!(event_location.id, location.id);
                    assert_eq!(device_posture_data, Some(posture_data));
                }
                other => panic!("unexpected bidi event: {other:?}"),
            },
            other => panic!("unexpected bidi stream event type: {other:?}"),
        }
        assert_eq!(event.context.user_id, user.id);
        assert_eq!(event.context.username, user.username);
        assert_eq!(event.context.ip, Some(DEVICE_INFO_IP.parse().unwrap()));
    }

    /// A failing posture evaluation must be auditable and revoke an existing posture session.
    #[sqlx::test]
    async fn test_posture_check_failure_revokes_active_session(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        set_enterprise_license();
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");
        let location = create_non_mfa_location(&pool).await;
        save_linux_posture_policy(&pool, location.id).await;
        let user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, device.id).await;
        let mut active_session = VpnClientSession::new(
            location.id,
            user.id,
            device.id,
            Some(Utc::now().naive_utc()),
            None,
        );
        active_session.preshared_key = Some("active-posture-psk".to_owned());
        active_session.state = VpnClientSessionState::Connected;
        let active_session = active_session
            .save(&pool)
            .await
            .expect("failed to create active posture session");
        let token = create_polling_token(&pool, device.id).await;
        let (mut server, mut event_rx, mut gateway_rx) = make_server(pool.clone());

        // the policy requires disk encryption
        let posture_data = DevicePostureData {
            disk_encryption: Some(BoolCheck {
                result: Some(bool_check::Result::Value(false)),
            }),
            ..passing_linux_posture_data()
        };
        let rejected_checks = match server
            .handle_posture_check(
                DevicePostureCheckRequest {
                    location_id: location.id,
                    pubkey: device.wireguard_pubkey.clone(),
                    device_posture_data: Some(posture_data.clone()),
                    token: Some(token),
                },
                device_info(),
            )
            .await
            .expect("posture check should complete")
        {
            super::PostureCheckOutcome::Approved { .. } => {
                panic!("posture check with unencrypted disk should be rejected")
            }
            super::PostureCheckOutcome::Rejected { failed_checks } => failed_checks,
        };
        assert!(!rejected_checks.is_empty());

        let event = event_rx
            .try_recv()
            .expect("expected posture check failed audit event");
        match event.event {
            BidiStreamEventType::DesktopClientMfa(event) => match *event {
                DesktopClientMfaEvent::PostureCheckFailed {
                    device: event_device,
                    location: event_location,
                    device_posture_data,
                    failed_checks,
                } => {
                    assert_eq!(event_device.id, device.id);
                    assert_eq!(event_location.id, location.id);
                    assert_eq!(device_posture_data, Some(posture_data));
                    assert_eq!(failed_checks, rejected_checks);
                }
                other => panic!("unexpected bidi event: {other:?}"),
            },
            other => panic!("unexpected bidi stream event type: {other:?}"),
        }
        assert_eq!(event.context.user_id, user.id);
        assert_eq!(event.context.username, user.username);

        match gateway_rx
            .try_recv()
            .expect("expected rejected posture session to be deauthorized")
        {
            GatewayCommand::VpnSessionDeauthorized(location_id, disconnected_device) => {
                assert_eq!(location_id, location.id);
                assert_eq!(disconnected_device.id, device.id);
            }
            other => panic!("unexpected gateway event: {other:?}"),
        }
        assert!(gateway_rx.try_recv().is_err());

        let event = event_rx
            .try_recv()
            .expect("expected session disconnected audit event");
        match event.event {
            BidiStreamEventType::DesktopClientMfa(event) => match *event {
                DesktopClientMfaEvent::Disconnected {
                    location: event_location,
                    device: event_device,
                    is_mfa_session,
                } => {
                    assert_eq!(event_location.id, location.id);
                    assert_eq!(event_device.id, device.id);
                    assert!(!is_mfa_session);
                }
                other => panic!("unexpected bidi event: {other:?}"),
            },
            other => panic!("unexpected bidi stream event type: {other:?}"),
        }
        assert_eq!(event.context.ip, Some(DEVICE_INFO_IP.parse().unwrap()));

        let active_session = VpnClientSession::find_by_id(&pool, active_session.id)
            .await
            .expect("failed to reload active posture session")
            .expect("expected active posture session");
        assert_eq!(active_session.state, VpnClientSessionState::Disconnected);
        assert!(active_session.disconnected_at.is_some());
        assert!(
            VpnClientSession::get_all_active_device_sessions_in_location(
                &pool,
                location.id,
                device.id
            )
            .await
            .expect("failed to query sessions")
            .is_empty()
        );
    }

    #[sqlx::test]
    async fn test_mfa_start_posture_failure_revokes_active_session(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        set_enterprise_license();
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");
        let location = create_mfa_location(&pool).await;
        save_linux_posture_policy(&pool, location.id).await;
        let user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, device.id).await;
        let mut active_session = VpnClientSession::new(
            location.id,
            user.id,
            device.id,
            Some(Utc::now().naive_utc()),
            Some(VpnClientMfaMethod::Totp),
        );
        active_session.preshared_key = Some("active-mfa-psk".to_owned());
        let active_session = active_session
            .save(&pool)
            .await
            .expect("failed to create active MFA session");
        let (mut server, mut event_rx, mut gateway_rx) = make_server(pool.clone());
        let posture_data = DevicePostureData {
            disk_encryption: Some(BoolCheck {
                result: Some(bool_check::Result::Value(false)),
            }),
            ..passing_linux_posture_data()
        };

        let outcome = server
            .start_client_mfa_login(
                ClientMfaStartRequest {
                    location_id: location.id,
                    pubkey: device.wireguard_pubkey.clone(),
                    method: MfaMethod::Email as i32,
                    posture_data: Some(posture_data),
                },
                device_info(),
            )
            .await
            .expect("posture check should complete");
        assert!(matches!(
            outcome,
            super::ClientMfaStartOutcome::Rejected { .. }
        ));

        match gateway_rx
            .try_recv()
            .expect("expected rejected MFA session to be deauthorized")
        {
            GatewayCommand::VpnSessionDeauthorized(location_id, disconnected_device) => {
                assert_eq!(location_id, location.id);
                assert_eq!(disconnected_device.id, device.id);
            }
            other => panic!("unexpected gateway event: {other:?}"),
        }

        event_rx
            .try_recv()
            .expect("expected posture check failed audit event");
        let event = event_rx
            .try_recv()
            .expect("expected session disconnected audit event");
        assert_eq!(event.context.ip, Some(DEVICE_INFO_IP.parse().unwrap()));

        let active_session = VpnClientSession::find_by_id(&pool, active_session.id)
            .await
            .expect("failed to reload active MFA session")
            .expect("expected active MFA session");
        assert_eq!(active_session.state, VpnClientSessionState::Disconnected);
        assert!(active_session.disconnected_at.is_some());
    }

    #[sqlx::test]
    async fn test_session_revocation_survives_unavailable_side_effect_consumers(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        set_enterprise_license();
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");
        let location = create_non_mfa_location(&pool).await;
        save_linux_posture_policy(&pool, location.id).await;
        let user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, device.id).await;
        let session = VpnClientSession::new(
            location.id,
            user.id,
            device.id,
            Some(Utc::now().naive_utc()),
            None,
        )
        .save(&pool)
        .await
        .expect("failed to create active posture session");
        let token = create_polling_token(&pool, device.id).await;
        let (mut server, event_rx, gateway_rx) = make_server(pool.clone());
        drop(event_rx);
        drop(gateway_rx);
        let posture_data = DevicePostureData {
            disk_encryption: Some(BoolCheck {
                result: Some(bool_check::Result::Value(false)),
            }),
            ..passing_linux_posture_data()
        };

        let outcome = server
            .handle_posture_check(
                DevicePostureCheckRequest {
                    location_id: location.id,
                    pubkey: device.wireguard_pubkey.clone(),
                    device_posture_data: Some(posture_data),
                    token: Some(token),
                },
                device_info(),
            )
            .await
            .expect("side-effect delivery must not prevent posture rejection");
        assert!(matches!(
            outcome,
            super::PostureCheckOutcome::Rejected { .. }
        ));

        let session = VpnClientSession::find_by_id(&pool, session.id)
            .await
            .expect("failed to reload session")
            .expect("expected session");
        assert_eq!(session.state, VpnClientSessionState::Disconnected);
    }

    #[sqlx::test]
    async fn test_replacing_connected_mfa_session_emits_session_superseded_event(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let location = create_mfa_location(&pool).await;
        let user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, device.id).await;
        let old_session = VpnClientSession::new(
            location.id,
            user.id,
            device.id,
            Some(Utc::now().naive_utc()),
            Some(VpnClientMfaMethod::Totp),
        )
        .save(&pool)
        .await
        .expect("failed to create existing MFA session");

        let (server, mut event_rx, mut gateway_rx) = make_server(pool.clone());
        let mut conn = pool.acquire().await.expect("failed to acquire connection");

        server
            .create_new_session(
                &mut conn,
                &location,
                &user,
                &device,
                Some(VpnClientMfaMethod::Totp),
                REPLACEMENT_MFA_PRESHARED_KEY.to_owned(),
            )
            .await
            .expect("should replace connected MFA session");

        let gateway_event = gateway_rx
            .try_recv()
            .expect("expected MFA gateway disconnect event for replaced connected session");
        match gateway_event {
            GatewayCommand::VpnSessionDeauthorized(location_id, disconnected_device) => {
                assert_eq!(location_id, location.id);
                assert_eq!(disconnected_device.id, device.id);
            }
            other => panic!("unexpected gateway event: {other:?}"),
        }

        let event = event_rx
            .try_recv()
            .expect("expected session replaced audit event for replaced connected session");
        match event.event {
            BidiStreamEventType::DesktopClientMfa(event) => match *event {
                DesktopClientMfaEvent::SessionSuperseded {
                    location: event_location,
                    device: event_device,
                    is_mfa_session,
                } => {
                    assert_eq!(event_location.id, location.id);
                    assert_eq!(event_device.id, device.id);
                    assert!(is_mfa_session);
                }
                other => panic!("unexpected bidi event: {other:?}"),
            },
            other => panic!("unexpected bidi stream event type: {other:?}"),
        }
        assert_eq!(event.context.user_id, user.id);
        assert_eq!(event.context.username, user.username);

        let old_session = VpnClientSession::find_by_id(&pool, old_session.id)
            .await
            .expect("failed to query old session")
            .expect("expected old session");
        assert_eq!(old_session.state, VpnClientSessionState::Disconnected);
    }

    #[sqlx::test]
    async fn test_replacing_new_mfa_session_marks_session_disconnected_without_disconnect_audit_event(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let location = create_mfa_location(&pool).await;
        let user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, device.id).await;
        let old_session = VpnClientSession::new(
            location.id,
            user.id,
            device.id,
            None,
            Some(VpnClientMfaMethod::Totp),
        )
        .save(&pool)
        .await
        .expect("failed to create existing new MFA session");

        let (server, mut event_rx, mut gateway_rx) = make_server(pool.clone());
        let mut conn = pool.acquire().await.expect("failed to acquire connection");

        server
            .create_new_session(
                &mut conn,
                &location,
                &user,
                &device,
                Some(VpnClientMfaMethod::Totp),
                REPLACEMENT_MFA_PRESHARED_KEY.to_owned(),
            )
            .await
            .expect("should replace new MFA session");

        let gateway_event = gateway_rx
            .try_recv()
            .expect("expected MFA gateway disconnect event for replaced new session");
        match gateway_event {
            GatewayCommand::VpnSessionDeauthorized(location_id, disconnected_device) => {
                assert_eq!(location_id, location.id);
                assert_eq!(disconnected_device.id, device.id);
            }
            other => panic!("unexpected gateway event: {other:?}"),
        }

        assert!(matches!(
            event_rx.try_recv(),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty)
        ));

        let old_session = VpnClientSession::find_by_id(&pool, old_session.id)
            .await
            .expect("failed to query old session")
            .expect("expected old session");
        assert_eq!(old_session.state, VpnClientSessionState::Disconnected);
    }

    fn make_server(
        pool: PgPool,
    ) -> (
        ClientMfaServer,
        tokio::sync::mpsc::UnboundedReceiver<BidiStreamEvent>,
        tokio::sync::broadcast::Receiver<GatewayCommand>,
    ) {
        let (gateway_tx, gateway_rx) = broadcast::channel(8);
        let (bidi_event_tx, bidi_event_rx) = mpsc::unbounded_channel();
        let remote_mfa_responses: Arc<RwLock<HashMap<String, oneshot::Sender<String>>>> =
            Arc::default();
        let sessions: Arc<RwLock<HashMap<String, ClientLoginSession>>> = Arc::default();

        (
            ClientMfaServer::new(
                pool,
                gateway_tx,
                bidi_event_tx,
                remote_mfa_responses,
                sessions,
            ),
            bidi_event_rx,
            gateway_rx,
        )
    }

    async fn create_user(pool: &PgPool) -> User<Id> {
        User::new(
            "client-mfa-test",
            Some("pass123"),
            "Tester",
            "ClientMfa",
            "client-mfa@example.com",
            None,
        )
        .save(pool)
        .await
        .expect("failed to create user")
    }

    async fn create_device(pool: &PgPool, user_id: Id) -> Device<Id> {
        Device::new(
            "client-mfa-device".to_owned(),
            "client-mfa-pubkey".to_owned(),
            user_id,
            DeviceType::User,
            None,
            true,
        )
        .save(pool)
        .await
        .expect("failed to create device")
    }

    /// Issues a polling token for a device, as enrollment does. Posture checks require one to
    /// authenticate the caller.
    async fn create_polling_token(pool: &PgPool, device_id: Id) -> String {
        PollingToken::new(device_id)
            .save(pool)
            .await
            .expect("failed to create polling token")
            .token
    }

    #[sqlx::test]
    async fn test_create_new_mfa_session_disconnects_previous_active_session(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let location = create_mfa_location(&pool).await;
        let user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, device.id).await;

        let mut previous_session = VpnClientSession::new(
            location.id,
            user.id,
            device.id,
            Some(Utc::now().naive_utc()),
            Some(VpnClientMfaMethod::Totp),
        );
        previous_session.preshared_key = Some("old-psk".to_owned());
        previous_session.state = VpnClientSessionState::Connected;
        let previous_session = previous_session
            .save(&pool)
            .await
            .expect("failed to create previous active MFA session");

        let (gateway_tx, mut gateway_rx) = broadcast::channel(4);
        let (bidi_event_tx, _bidi_event_rx) = mpsc::unbounded_channel();
        let server = ClientMfaServer::new(
            pool.clone(),
            gateway_tx,
            bidi_event_tx,
            Arc::new(RwLock::new(
                HashMap::<String, oneshot::Sender<String>>::new(),
            )),
            Arc::new(RwLock::new(HashMap::<String, ClientLoginSession>::new())),
        );
        let mut conn = pool
            .acquire()
            .await
            .expect("failed to acquire database connection");

        let new_session = server
            .create_new_session(
                &mut conn,
                &location,
                &user,
                &device,
                Some(VpnClientMfaMethod::Totp),
                NEW_MFA_PRESHARED_KEY.to_owned(),
            )
            .await
            .expect("failed to create replacement MFA session");

        let previous_session = VpnClientSession::find_by_id(&pool, previous_session.id)
            .await
            .expect("failed to reload previous session")
            .expect("expected previous session to exist");
        assert_eq!(previous_session.state, VpnClientSessionState::Disconnected);
        assert!(previous_session.disconnected_at.is_some());

        let active_sessions = VpnClientSession::get_all_active_device_sessions_in_location(
            &pool,
            location.id,
            device.id,
        )
        .await
        .expect("failed to fetch active sessions");
        assert_eq!(active_sessions.len(), 1);
        assert_eq!(active_sessions[0].id, new_session.id);
        assert_eq!(
            active_sessions[0].preshared_key.as_deref(),
            Some(NEW_MFA_PRESHARED_KEY)
        );

        match gateway_rx.try_recv() {
            Ok(GatewayCommand::VpnSessionDeauthorized(location_id, disconnected_device)) => {
                assert_eq!(location_id, location.id);
                assert_eq!(disconnected_device.id, device.id);
            }
            Ok(other) => panic!("unexpected gateway event: {other:?}"),
            Err(error) => panic!("expected MFA disconnect gateway event, got {error:?}"),
        }
    }

    async fn create_mfa_location(pool: &PgPool) -> WireguardNetwork<Id> {
        WireguardNetwork::new(
            "client-mfa-location".to_owned(),
            51820,
            "vpn.example.com".to_owned(),
            None,
            [IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap()],
            true,
            false,
            false,
            false,
            true, // mfa_enabled
            ServiceLocationMode::Disabled,
        )
        .set_address([IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 10, 0, 1)), 24).unwrap()])
        .expect("failed to set location address")
        .save(pool)
        .await
        .expect("failed to create location")
    }

    async fn create_non_mfa_location(pool: &PgPool) -> WireguardNetwork<Id> {
        WireguardNetwork::new(
            "client-posture-location".to_owned(),
            51820,
            "vpn.example.com".to_owned(),
            None,
            [IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap()],
            true,
            false,
            false,
            false,
            false, // mfa_enabled
            ServiceLocationMode::Disabled,
        )
        .set_address([IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 20, 0, 1)), 24).unwrap()])
        .expect("failed to set location address")
        .save(pool)
        .await
        .expect("failed to create location")
    }

    async fn attach_device_to_location(pool: &PgPool, location_id: Id, device_id: Id) {
        WireguardNetworkDevice::new(
            location_id,
            device_id,
            vec![IpAddr::V4(Ipv4Addr::new(10, 10, 0, 10))],
        )
        .insert(pool)
        .await
        .expect("failed to attach device to location");
    }

    fn set_enterprise_license() {
        let license = License::new(
            "test".to_owned(),
            true,
            Some(Utc::now() + chrono::TimeDelta::days(1)),
            Some(LicenseLimits {
                users: 100,
                devices: 100,
                locations: 100,
                network_devices: Some(100),
            }),
            None,
            LicenseTier::Enterprise,
            SupportType::Basic,
            vec![],
        );
        set_cached_license(Some(license));
        set_counts(Counts::new(1, 1, 1, 1));
    }

    fn passing_linux_posture_data() -> DevicePostureData {
        DevicePostureData {
            defguard_client_version: "1.6.0".to_owned(),
            os_type: "linux".to_owned(),
            disk_encryption: Some(BoolCheck {
                result: Some(bool_check::Result::Value(true)),
            }),
            ..Default::default()
        }
    }

    async fn save_linux_posture_policy(pool: &PgPool, location_id: Id) {
        let policy = DevicePosture {
            id: defguard_common::db::NoId,
            name: "client-mfa-test-posture".to_owned(),
            description: None,
            min_desktop_client_version: None,
            min_mobile_client_version: None,
            allow_prerelease_client: true,
        }
        .save(pool)
        .await
        .expect("failed to save posture policy");

        DevicePostureOsRule {
            id: defguard_common::db::NoId,
            posture_id: policy.id,
            os_type: OsType::Linux,
            min_os_version: None,
            disk_encryption_required: Some(true),
            antivirus_required: None,
            ad_domain_joined_required: None,
            windows_security_update_max_age: None,
            min_kernel_version: None,
            device_integrity_required: None,
            android_security_patch_level_max_age: None,
        }
        .save(pool)
        .await
        .expect("failed to save posture OS rule");

        DevicePostureLocation::set_for_location(
            &mut pool.acquire().await.expect("failed to acquire connection"),
            location_id,
            &[policy.id],
        )
        .await
        .expect("failed to assign posture policy to location");
    }
}
