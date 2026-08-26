use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, RwLock},
    time::Duration,
};

use chrono::Utc;
use defguard_common::{
    db::{
        Id,
        models::{
            Device, User, WireguardNetwork,
            device::WireguardNetworkDevice,
            mfa_flow::{MfaFlow, MfaFlowStep},
            polling_token::PollingToken,
            vpn_client_mfa_session::{VpnClientMfaSession, hash_token},
            vpn_client_session::{VpnClientMfaMethod, VpnClientSession, VpnClientSessionState},
        },
    },
    types::user_info::UserInfo,
};
use defguard_proto::{
    client_types::{
        ClientMfaFinishRequest, ClientMfaFinishResponse, ClientMfaStartRequest,
        ClientMfaStartResponse, ClientMfaStepStartRequest, ClientMfaStepStartResponse, MfaAdvanced,
        MfaCompleted, MfaMethod, MfaStartRejectionReason, MfaStepRejection, MfaStepResult,
        mfa_step_result,
    },
    enterprise::posture::DevicePostureCheckRequest,
    proxy::{
        self, AwaitRemoteMfaFinishRequest, AwaitRemoteMfaFinishResponse,
        ClientMfaTokenValidationRequest, ClientMfaTokenValidationResponse, CoreResponse,
        core_response::Payload,
    },
};
use sqlx::{PgConnection, PgPool, Postgres, pool::PoolConnection};
use tokio::{
    sync::{broadcast::Sender, mpsc::UnboundedSender, oneshot},
    time,
};
use tonic::{Code, Status};

use crate::{
    enterprise::{
        is_business_license_active,
        posture::{PostureCheckError, PostureResult, validate_posture},
    },
    events::{BidiRequestContext, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent},
    grpc::{GatewayCommand, utils::parse_client_ip_agent},
    mfa_engine::{
        MfaEngine,
        authorize::{
            AuthorizeError, ClientMfaServerError, EventChannels,
            build_authorized_gateway_network_info, create_new_session,
        },
        error::{FinishError, StartError, StepError},
        method::InitiateError,
        types::{
            FinishOutcome, Proof, StartOutcome, StartRejectionReason, StartResult, StepRejection,
            StepStarted,
        },
    },
};

// How much time the user has to approve remote MFA with mobile device
const REMOTE_AUTH_TIMEOUT: Duration = Duration::from_mins(1);

pub struct ClientMfaServer {
    pub(crate) pool: PgPool,
    channels: EventChannels,
    remote_mfa_responses: Arc<RwLock<HashMap<String, oneshot::Sender<String>>>>,
    engine: MfaEngine,
}

/// Acquire a pooled connection, mapping a pool error to an internal status.
async fn acquire_connection(pool: &PgPool) -> Result<PoolConnection<Postgres>, Status> {
    pool.acquire().await.map_err(|_| {
        error!("Failed to acquire DB connection");
        Status::internal("unexpected error")
    })
}

/// Remove a remote-MFA waiter from the map, dropping the entry so a never-finishing client or a
/// dropped sender cannot leak a map entry.
fn remove_remote_mfa_waiter(
    waiters: &Arc<RwLock<HashMap<String, oneshot::Sender<String>>>>,
    hash: &str,
) {
    waiters
        .write()
        .expect("Failed to write-lock ClientMfaServer::remote_mfa_responses")
        .remove(hash);
}

impl From<ClientMfaServerError> for Status {
    fn from(value: ClientMfaServerError) -> Self {
        Self::new(Code::Internal, value.to_string())
    }
}

impl From<AuthorizeError> for Status {
    fn from(err: AuthorizeError) -> Self {
        match err {
            AuthorizeError::Db(_) | AuthorizeError::Gateway(_) => {
                Status::internal("unexpected error")
            }
            AuthorizeError::Event(e) => Status::from(e),
        }
    }
}

impl From<InitiateError> for Status {
    fn from(err: InitiateError) -> Self {
        match err {
            InitiateError::EmailCode(_) => Status::internal("MFA code"),
            InitiateError::Database(_) => Status::internal("database error"),
            InitiateError::Mail(_) => Status::internal("unexpected error"),
            InitiateError::BiometricNotConfigured => {
                Status::invalid_argument("Select MFA method is not available for the device.")
            }
            InitiateError::InvalidPublicKey(_) => Status::invalid_argument("Invalid public key"),
        }
    }
}

// The engine's domain types carry no proto, so the conversions live here rather than in
// `mfa_engine`.

impl From<FinishOutcome> for MfaStepResult {
    fn from(value: FinishOutcome) -> Self {
        let outcome = match value {
            FinishOutcome::Advanced { next_step } => {
                mfa_step_result::Outcome::Advanced(MfaAdvanced { next_step })
            }
            FinishOutcome::Completed { preshared_key } => {
                mfa_step_result::Outcome::Completed(MfaCompleted { preshared_key })
            }
        };
        MfaStepResult {
            outcome: Some(outcome),
        }
    }
}

impl From<StepStarted> for ClientMfaStepStartResponse {
    fn from(value: StepStarted) -> Self {
        Self {
            step_attempt_id: value.step_attempt_id,
            challenge: value.challenge,
        }
    }
}

impl From<StartRejectionReason> for MfaStartRejectionReason {
    fn from(value: StartRejectionReason) -> Self {
        match value {
            StartRejectionReason::MethodNotInStep => Self::MfaStartRejectionMethodNotInStep,
            StartRejectionReason::StepEmptyAfterLicense => {
                Self::MfaStartRejectionStepEmptyAfterLicense
            }
            StartRejectionReason::StepUnavailable => Self::MfaStartRejectionStepUnavailable,
        }
    }
}

impl From<StepRejection> for MfaStepRejection {
    fn from(value: StepRejection) -> Self {
        Self {
            step: value.step,
            reason: MfaStartRejectionReason::from(value.reason) as i32,
        }
    }
}

// The status message always comes from the variant's `Display` (the `#[error(...)]` attribute in
// `mfa_engine::error`), so each client-visible string exists in exactly one place and the only
// decision made here is the gRPC code. Editing a `Display` string changes the client contract.

impl From<StartError> for Status {
    fn from(err: StartError) -> Self {
        let code = match err {
            StartError::MultiStepNotAvailable => Code::FailedPrecondition,
            StartError::PlanLengthMismatch
            | StartError::MethodNotAvailable
            | StartError::BiometricNotConfigured => Code::InvalidArgument,
            StartError::Internal => Code::Internal,
            StartError::Initiate(e) => return Status::from(e),
        };
        Status::new(code, err.to_string())
    }
}

impl From<StepError> for Status {
    fn from(err: StepError) -> Self {
        let code = match err {
            StepError::SessionNotFound | StepError::MethodNotInStep => Code::InvalidArgument,
            StepError::MethodNotConfigured => Code::FailedPrecondition,
            StepError::Internal => Code::Internal,
            StepError::Initiate(e) => return Status::from(e),
        };
        Status::new(code, err.to_string())
    }
}

impl From<FinishError> for Status {
    fn from(err: FinishError) -> Self {
        let code = match err {
            FinishError::SessionNotFound
            | FinishError::UninitializedStep
            | FinishError::StaleAttempt
            | FinishError::MissingChallenge
            | FinishError::MalformedProof { .. } => Code::InvalidArgument,
            FinishError::OidcNotCompleted => Code::FailedPrecondition,
            FinishError::Unauthorized => Code::Unauthenticated,
            FinishError::MissingBiometricChallenge | FinishError::Internal => Code::Internal,
            FinishError::Event(e) => return Status::from(e),
        };
        Status::new(code, err.to_string())
    }
}

impl ClientMfaServer {
    #[must_use]
    pub fn new(
        pool: PgPool,
        gateway_tx: Sender<GatewayCommand>,
        bidi_event_tx: UnboundedSender<BidiStreamEvent>,
        remote_mfa_responses: Arc<RwLock<HashMap<String, oneshot::Sender<String>>>>,
    ) -> Self {
        let engine = MfaEngine::new(pool.clone(), gateway_tx.clone(), bidi_event_tx.clone());
        Self {
            pool,
            channels: EventChannels::new(gateway_tx, bidi_event_tx),
            remote_mfa_responses,
            engine,
        }
    }

    pub(crate) fn emit_event(&self, event: BidiStreamEvent) -> Result<(), ClientMfaServerError> {
        self.channels.emit_event(event)
    }

    /// Acquire a pooled connection, mapping a pool error to an internal status.
    pub(crate) async fn acquire_conn(&self) -> Result<PoolConnection<Postgres>, Status> {
        acquire_connection(&self.pool).await
    }

    /// Allows Edge to verify if token is valid and active.
    #[instrument(skip_all)]
    pub async fn validate_mfa_token(
        &mut self,
        request: ClientMfaTokenValidationRequest,
    ) -> Result<ClientMfaTokenValidationResponse, Status> {
        let token_valid =
            VpnClientMfaSession::<Id>::find_active_by_token(&self.pool, &request.token)
                .await
                .map_err(|err| {
                    error!("Failed to validate MFA token: {err}");
                    Status::internal("unexpected error")
                })?
                .is_some();
        Ok(ClientMfaTokenValidationResponse { token_valid })
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
        if !location.mfa_enabled {
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

        // Parse the caller's device info before the posture block, the first thing here that can
        // write. Rejecting it later would leave a live session row behind, and on the supersede
        // path would already have torn down the caller's previous session. It stays after the
        // entity lookups so their more specific `not found` errors keep precedence.
        let (ip, _user_agent) = parse_client_ip_agent(&info).map_err(Status::internal)?;

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

        // A non-empty `selected_methods` is the capability proof that the client speaks the
        // multi-step protocol; an empty list falls back to the deprecated single-method field.
        if request.selected_methods.is_empty() {
            // Legacy single-step path.
            #[allow(deprecated)]
            let selected_method = MfaMethod::try_from(request.method).map_err(|err| {
                error!("Invalid MFA method selected ({}): {err}", request.method);
                Status::invalid_argument("invalid MFA method selected")
            })?;

            // Reject locations whose flow configuration cannot be expressed as a legacy
            // single-factor mode (multi-flow, multi-step, or a subset of the internal method set).
            // Fail closed rather than silently driving only the first step.
            if MfaFlow::derive_legacy_mode(&self.pool, location.id)
                .await
                .map_err(|err| {
                    error!("Failed to derive legacy MFA mode: {err}");
                    Status::internal("unexpected error")
                })?
                .is_none()
            {
                error!(
                    "Location {location} has an MFA flow configuration that cannot be enforced by \
                    this client"
                );
                return Err(Status::failed_precondition(
                    "Defguard client version is too old to connect to this location. Please update your client.",
                ));
            }

            // The legacy adapter drives only the first step, so license-filter that step's methods
            // and validate the client's selected method against them.
            let (flow, steps) = self
                .resolve_mfa_flow(
                    &location,
                    &user,
                    "location MFA configuration is not supported by this client",
                )
                .await?;

            let Some(first_step) = steps.first() else {
                error!("Resolved MFA flow has no steps");
                return Err(Status::internal("unexpected error"));
            };
            let first_step_methods: Vec<VpnClientMfaMethod> = first_step
                .methods
                .iter()
                .copied()
                // OIDC MFA is a business feature, so an unlicensed deployment must not offer it.
                .filter(|method| {
                    *method != VpnClientMfaMethod::Oidc || is_business_license_active()
                })
                .collect();

            let selected_client_method: VpnClientMfaMethod = selected_method.into();
            if !first_step_methods.contains(&selected_client_method) {
                error!(
                    "Selected MFA method ({selected_method}) is not supported by location \
                    {location}"
                );
                return Err(Status::invalid_argument(
                    "selected MFA method is not supported by location",
                ));
            }

            let start_outcome = self
                .engine
                .start(
                    &location,
                    &device,
                    &user,
                    flow.id,
                    vec![first_step_methods],
                    selected_client_method,
                )
                .await?;

            self.finish_start(start_outcome, &user, ip, &device, &location)
                .await
        } else {
            // Multi-step path.
            let selected_methods: Vec<VpnClientMfaMethod> = request
                .selected_methods
                .iter()
                .map(|&method| {
                    MfaMethod::try_from(method)
                        .map(VpnClientMfaMethod::from)
                        .map_err(|err| {
                            error!("Invalid MFA method selected ({method}): {err}");
                            Status::invalid_argument("invalid MFA method selected")
                        })
                })
                .collect::<Result<_, _>>()?;

            let (flow, steps) = self
                .resolve_mfa_flow(
                    &location,
                    &user,
                    "no MFA flow applies to this user and location",
                )
                .await?;

            let step_methods: Vec<Vec<VpnClientMfaMethod>> =
                steps.into_iter().map(|step| step.methods).collect();

            match self
                .engine
                .start_multi_step(
                    &location,
                    &device,
                    &user,
                    flow.id,
                    step_methods,
                    selected_methods,
                )
                .await?
            {
                StartResult::Accepted(start_outcome) => {
                    self.finish_start(start_outcome, &user, ip, &device, &location)
                        .await
                }
                StartResult::Rejected(rejections) => {
                    info!(
                        "MFA plan rejected for user {} at location {}: {rejections:?}",
                        user.username, location.name
                    );
                    Ok(ClientMfaStartOutcome::Approved(ClientMfaStartResponse {
                        token: String::new(),
                        challenge: None,
                        rejections: rejections.into_iter().map(Into::into).collect(),
                    }))
                }
            }
        }
    }

    /// Handle an accepted start: cancel the superseded waiter, emit the supersede event, and build
    /// the response.
    async fn finish_start(
        &self,
        start_outcome: StartOutcome,
        user: &User<Id>,
        ip: IpAddr,
        device: &Device<Id>,
        location: &WireguardNetwork<Id>,
    ) -> Result<ClientMfaStartOutcome, Status> {
        if let Some(superseded_token_hash) = start_outcome.superseded_token_hash {
            self.remote_mfa_responses
                .write()
                .expect("Failed to write-lock ClientMfaServer::remote_mfa_responses")
                .remove(&superseded_token_hash);

            let context =
                BidiRequestContext::new(user.id, user.username.clone(), ip, device.name.clone());
            self.emit_event(BidiStreamEvent {
                context,
                event: BidiStreamEventType::DesktopClientMfa(Box::new(
                    DesktopClientMfaEvent::MfaLoginSuperseded {
                        location: location.clone(),
                        device: device.clone(),
                    },
                )),
            })?;
        }

        info!(
            "Desktop client MFA login started for {} at location {}",
            user.username, location.name
        );

        Ok(ClientMfaStartOutcome::Approved(ClientMfaStartResponse {
            token: start_outcome.token,
            challenge: start_outcome.challenge,
            rejections: Vec::new(),
        }))
    }

    /// Resolve the MFA flow applying to `user` at `location`. The two start paths differ only in
    /// the message reported when no flow applies, hence `no_flow_message`.
    async fn resolve_mfa_flow(
        &self,
        location: &WireguardNetwork<Id>,
        user: &User<Id>,
        no_flow_message: &'static str,
    ) -> Result<(MfaFlow<Id>, Vec<MfaFlowStep<Id>>), Status> {
        let mut conn = self.acquire_conn().await?;
        match MfaFlow::resolve_for_user(&mut conn, location.id, user.id).await {
            Ok(Some((flow, steps))) => Ok((flow, steps)),
            Ok(None) => {
                error!(
                    "Location {location} has no MFA flow that applies to user {}",
                    user.username
                );
                Err(Status::failed_precondition(no_flow_message))
            }
            Err(err) => {
                error!("Failed to resolve MFA flow: {err}");
                Err(Status::internal("unexpected error"))
            }
        }
    }

    /// Checks whether the user and device are allowed to access a location.
    async fn validate_location_access(
        pool: &PgPool,
        location: &WireguardNetwork<Id>,
        device: &Device<Id>,
        user_info: &UserInfo,
    ) -> Result<(), Status> {
        // acquire connection
        let mut conn = acquire_connection(pool).await?;

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
        debug!("Awaiting remote MFA finish for request_id {request_id}");

        // Register a waiter only for a token that maps to a live in-progress session, so an
        // unauthenticated caller cannot grow the waiter map without bound.
        if VpnClientMfaSession::<Id>::find_active_by_token(&self.pool, &request.token)
            .await
            .map_err(|err| {
                error!("Failed to find MFA session: {err}");
                Status::internal("unexpected error")
            })?
            .is_none()
        {
            error!("Client login session not found");
            return Err(Status::invalid_argument("login session not found"));
        }

        let hash = hash_token(&request.token);
        let (tx, rx) = oneshot::channel();
        self.remote_mfa_responses
            .write()
            .expect("Failed to write-lock ClientMfaServer::remote_mfa_responses")
            .insert(hash.clone(), tx);

        let waiters = self.remote_mfa_responses.clone();
        // Spawn a task that waits for remote MFA process to conclude to get the preshared key.
        tokio::spawn(async move {
            match time::timeout(REMOTE_AUTH_TIMEOUT, rx).await {
                Ok(Ok(preshared_key)) => {
                    let req = CoreResponse {
                        id: request_id,
                        payload: Some(Payload::AwaitRemoteMfaFinish(
                            AwaitRemoteMfaFinishResponse {
                                #[allow(deprecated)]
                                preshared_key,
                                result: None,
                            },
                        )),
                    };
                    // Once the key is here, send it back to proxy.
                    let _ = response_tx.send(req);
                }
                Ok(Err(err)) => {
                    // Drop the waiter so a dropped sender cannot leak a map entry.
                    remove_remote_mfa_waiter(&waiters, &hash);
                    error!("Remote MFA response channel failed: {err:?}");
                }
                Err(_) => {
                    // Drop the waiter so a client that never finishes cannot leak map entries.
                    remove_remote_mfa_waiter(&waiters, &hash);
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
        debug!("Finishing desktop client login");

        let token = request.token.clone();
        let proof = Proof {
            code: request.code,
            auth_pub_key: request.auth_pub_key,
            step_attempt_id: request.step_attempt_id,
            auth_data: request.auth_data,
        };
        let (ip, _user_agent) = parse_client_ip_agent(&info).map_err(Status::internal)?;

        let (outcome, method) = self.engine.finish(token.clone(), proof, ip).await?;

        // The parked remote-MFA waiter is session-scoped, so resolve it only once the flow
        // completes. An intermediate step (`Advanced`) must not terminate it: a pre-2.2 client
        // reads an empty preshared key as success.
        let preshared_key = match &outcome {
            FinishOutcome::Completed { preshared_key } => {
                if let Some(tx) = self
                    .remote_mfa_responses
                    .write()
                    .expect("Failed to write-lock ClientMfaServer::remote_mfa_responses")
                    .remove(&hash_token(&token))
                {
                    let _ = tx.send(preshared_key.clone());
                }
                preshared_key.clone()
            }
            FinishOutcome::Advanced { .. } => String::new(),
        };

        let response = ClientMfaFinishResponse {
            #[allow(deprecated)]
            preshared_key: preshared_key.clone(),
            token: match method {
                VpnClientMfaMethod::MobileApprove => Some(token),
                _ => None,
            },
            result: Some(outcome.into()),
        };

        Ok(response)
    }

    #[instrument(skip_all)]
    pub async fn client_mfa_step_start(
        &mut self,
        request: ClientMfaStepStartRequest,
    ) -> Result<ClientMfaStepStartResponse, Status> {
        let method = MfaMethod::try_from(request.method)
            .map(VpnClientMfaMethod::from)
            .map_err(|err| {
                error!("Invalid MFA method selected ({}): {err}", request.method);
                Status::invalid_argument("invalid MFA method selected")
            })?;
        let step_started = self.engine.step_start(request.token, method).await?;
        Ok(ClientMfaStepStartResponse::from(step_started))
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

        if location.mfa_enabled {
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
            build_authorized_gateway_network_info(network_device, key.public.clone());

        create_new_session(
            &self.channels,
            &mut transaction,
            &location,
            &user,
            &device,
            false,
            key.public.clone(),
        )
        .await?;

        transaction.commit().await.map_err(|err| {
            error!("Failed to commit transaction for posture session: {err}");
            Status::internal("unexpected error")
        })?;

        let event =
            GatewayCommand::VpnSessionAuthorized(location.id, device.clone(), gateway_network_info);
        self.channels.gateway_tx.send(event).map_err(|err| {
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
        if let Err(err) = self.channels.gateway_tx.send(event) {
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
            let is_mfa_session = session.is_mfa_session;
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
mod tests;
