use std::collections::HashSet;

use defguard_common::{
    config::server_config,
    csv::AsCsv,
    db::{
        Id,
        models::{
            BiometricAuth, Device, DeviceConfig, DeviceType, MFAMethod, Settings, User,
            WireguardNetwork, device::DeviceInfo, polling_token::PollingToken,
            wireguard::ServiceLocationMode,
        },
    },
};
use defguard_core::{
    db::models::enrollment::{ENROLLMENT_TOKEN_TYPE, Token},
    enterprise::{
        db::models::{enterprise_settings::EnterpriseSettings, openid_provider::OpenIdProvider},
        firewall::try_get_location_firewall_config,
        ldap::utils::ldap_add_user,
        limits::update_counts,
    },
    events::{BidiRequestContext, BidiStreamEvent, BidiStreamEventType, EnrollmentEvent},
    grpc::{
        InstanceInfo,
        client_version::ClientFeature,
        gateway::events::GatewayEvent,
        utils::{build_device_config_response, new_polling_token, parse_client_ip_agent},
    },
    handlers::{
        mail::{
            send_email_mfa_activation_email, send_mfa_configured_email, send_new_device_added_email,
        },
        user::check_password_strength,
    },
    headers::get_device_info,
    is_valid_phone_number,
};
use defguard_mail::{Mail, templates::TemplateLocation};
use defguard_proto::proxy::{
    ActivateUserRequest, AdminInfo, CodeMfaSetupFinishRequest, CodeMfaSetupFinishResponse,
    CodeMfaSetupStartRequest, CodeMfaSetupStartResponse, DeviceConfigResponse,
    EnrollmentStartRequest, EnrollmentStartResponse, ExistingDevice, InitialUserInfo, MfaMethod,
    NewDevice, RegisterMobileAuthRequest,
};
use sqlx::{PgPool, query_scalar};
use tokio::sync::{
    broadcast::Sender,
    mpsc::{UnboundedSender, error::SendError},
};
use tonic::Status;

pub(crate) struct EnrollmentServer {
    pool: PgPool,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
    bidi_event_tx: UnboundedSender<BidiStreamEvent>,
}

impl EnrollmentServer {
    #[must_use]
    pub(crate) fn new(
        pool: PgPool,
        wireguard_tx: Sender<GatewayEvent>,
        mail_tx: UnboundedSender<Mail>,
        bidi_event_tx: UnboundedSender<BidiStreamEvent>,
    ) -> Self {
        Self {
            pool,
            wireguard_tx,
            mail_tx,
            bidi_event_tx,
        }
    }

    /// Checks if token provided with request corresponds to a valid enrollment session
    async fn validate_session(&self, token: Option<&String>) -> Result<Token, Status> {
        info!("Validating enrollment session. Token: {token:?}");
        let Some(token) = token else {
            error!("Missing authorization header in request");
            return Err(Status::unauthenticated("Missing authorization header"));
        };
        let enrollment = Token::find_by_id(&self.pool, token).await?;
        debug!("Found matching token, verifying validity: {enrollment:?}.");
        if enrollment
            .token_type
            .as_ref()
            .is_none_or(|token_type| token_type != ENROLLMENT_TOKEN_TYPE)
        {
            error!(
                "Invalid token type used in enrollment process: {:?}",
                enrollment.token_type
            );
            return Err(Status::permission_denied("invalid token"));
        }
        if enrollment.is_session_valid(server_config().enrollment_session_timeout.as_secs()) {
            info!("Enrollment session validated: {enrollment:?}");
            Ok(enrollment)
        } else {
            error!("Enrollment session expired: {enrollment:?}");
            Err(Status::unauthenticated("Session expired"))
        }
    }

    /// Sends given `GatewayEvent` to be handled by gateway GRPC server
    pub(crate) fn send_wireguard_event(&self, event: GatewayEvent) {
        if let Err(err) = self.wireguard_tx.send(event) {
            error!("Error sending WireGuard event {err}");
        }
    }

    // Send event to the dedicated bidi stream event channel
    fn emit_event(
        &self,
        context: BidiRequestContext,
        event: EnrollmentEvent,
    ) -> Result<(), SendError<BidiStreamEvent>> {
        let event = BidiStreamEvent {
            context,
            event: BidiStreamEventType::Enrollment(Box::new(event)),
        };

        self.bidi_event_tx.send(event)
    }

    #[instrument(skip_all)]
    pub(crate) async fn start_enrollment(
        &self,
        request: EnrollmentStartRequest,
        info: Option<defguard_proto::proxy::DeviceInfo>,
    ) -> Result<EnrollmentStartResponse, Status> {
        debug!("Starting enrollment session, request: {request:?}");
        // fetch enrollment token
        debug!("Try to find an enrollment token {}.", request.token);
        let mut enrollment = Token::find_by_id(&self.pool, &request.token).await?;

        if let Some(token_type) = &enrollment.token_type {
            if token_type != ENROLLMENT_TOKEN_TYPE {
                error!("Invalid token type used while trying to start enrollment: {token_type}");
                return Err(Status::permission_denied("invalid token"));
            }

            // fetch related users
            let user = enrollment.fetch_user(&self.pool).await?;
            let admin = enrollment.fetch_admin(&self.pool).await?;

            debug!(
                "Checking if user {}({:?}) is active",
                user.username, user.id
            );
            if !user.is_active {
                warn!(
                    "Can't start enrollment for disabled user {}.",
                    user.username
                );
                return Err(Status::permission_denied("user is disabled"));
            }
            info!(
                "User {}({:?}) is active, proceeding with enrollment",
                user.username, user.id
            );

            let mut transaction = self.pool.begin().await.map_err(|err| {
                error!("Failed to begin a transaction for enrollment: {err}");
                Status::internal("unexpected error")
            })?;

            // validate token & start session
            debug!(
                "Validating enrollment token and starting session for user {}({:?})",
                user.username, user.id,
            );
            let session_deadline = enrollment
                .start_session(
                    &mut transaction,
                    server_config().enrollment_session_timeout.as_secs(),
                )
                .await?;
            info!(
                "Enrollment session started for user {}({:?})",
                user.username, user.id
            );

            debug!(
                "Retrieving settings for enrollment of user {}({:?}).",
                user.username, user.id
            );
            let settings = Settings::get_current_settings();
            debug!("Settings: {settings:?}");

            debug!(
                "Retrieving enterprise settings for enrollment of user {}({:?}).",
                user.username, user.id
            );
            let enterprise_settings =
                EnterpriseSettings::get(&mut *transaction)
                    .await
                    .map_err(|err| {
                        error!("Failed to get enterprise settings: {err}");
                        Status::internal("unexpected error")
                    })?;
            debug!("Enterprise settings: {enterprise_settings:?}");

            let vpn_setup_optional = settings.enrollment_vpn_step_optional;
            debug!(
                "Retrieving instance info for user {}({:?}).",
                user.username, user.id
            );

            let openid_provider = OpenIdProvider::get_current(&self.pool)
                .await
                .map_err(|err| {
                    error!("Failed to get OpenID provider: {err}");
                    Status::internal(format!("unexpected error: {err}"))
                })?;
            let smtp_configured = settings.smtp_configured();
            let instance_info = InstanceInfo::new(
                settings,
                &user.username,
                &enterprise_settings,
                openid_provider,
            )
            .map_err(|err| {
                error!("Failed to create instance info: {err}");
                Status::internal("unexpected error")
            })?;
            debug!("Instance info {instance_info:?}");

            debug!(
                "Preparing initial user info to send for user enrollment, user {}({:?}).",
                user.username, user.id
            );
            let (username, user_id) = (user.username.clone(), user.id);
            let user_info = initial_info_from_user(&self.pool, user)
                .await
                .map_err(|err| {
                    error!(
                        "Failed to get user info for user {}({:?}): {err}",
                        username, user_id,
                    );
                    Status::internal("unexpected error")
                })?;
            debug!("User info {user_info:?}");

            debug!("Trying to get basic admin info...");
            let admin_info = admin.map(AdminInfo::from);
            debug!("Admin info {admin_info:?}");

            debug!("Creating enrollment start response for user {username}({user_id:?}).");
            let enterprise_settings =
                EnterpriseSettings::get(&mut *transaction)
                    .await
                    .map_err(|err| {
                        error!("Failed to get enterprise settings: {err}");
                        Status::internal("unexpected error")
                    })?;
            // check if any locations enforce internal MFA
            let instance_has_internal_mfa = query_scalar!(
                "SELECT EXISTS( \
                    SELECT 1 FROM wireguard_network \
                    WHERE location_mfa_mode = 'internal'::location_mfa_mode \
                ) \"exists!\""
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|_| Status::internal("Failed to read data".to_string()))?;
            let enrollment_settings = defguard_proto::proxy::EnrollmentSettings {
                vpn_setup_optional,
                smtp_configured,
                only_client_activation: enterprise_settings.only_client_activation,
                admin_device_management: enterprise_settings.admin_device_management,
                mfa_required: instance_has_internal_mfa,
            };
            let response = defguard_proto::proxy::EnrollmentStartResponse {
                admin: admin_info,
                user: Some(user_info),
                deadline_timestamp: session_deadline.and_utc().timestamp(),
                final_page_content: enrollment
                    .get_welcome_page_content(&mut transaction)
                    .await?,
                instance: Some(instance_info.into()),
                settings: Some(enrollment_settings),
            };
            debug!("Response {response:?}");

            transaction.commit().await.map_err(|err| {
                error!("Failed to commit transaction: {err}");
                Status::internal("unexpected error")
            })?;

            // Prepare event context and push the event
            let (ip, user_agent) = parse_client_ip_agent(&info).map_err(Status::internal)?;
            let context = BidiRequestContext::new(user_id, username, ip, user_agent);
            self.emit_event(context, EnrollmentEvent::EnrollmentStarted)
                .map_err(|err| {
                    error!("Failed to send event. Reason: {err}",);
                    Status::internal("unexpected error")
                })?;

            Ok(response)
        } else {
            debug!("Invalid enrollment token, the token does not have specified type.");
            Err(Status::permission_denied("invalid token"))
        }
    }

    #[instrument(skip_all)]
    pub(crate) async fn register_mobile_auth(
        &self,
        request: RegisterMobileAuthRequest,
    ) -> Result<(), Status> {
        debug!("Register mobile auth started");
        let enrollment = self.validate_session(Some(&request.token)).await?;
        let user = enrollment.fetch_user(&self.pool).await?;
        Device::validate_pubkey(&request.device_pub_key).map_err(|err| {
            error!(
                "Invalid public key {}, device won't be registered as mobile MFA auth for user {}\
                ({:?}): {err}",
                request.device_pub_key, user.username, user.id
            );
            Status::invalid_argument("invalid pubkey")
        })?;
        let Some(device) = Device::find_by_pubkey(&self.pool, &request.device_pub_key)
            .await
            .map_err(|err| {
                error!("Failed to read devices from db: {err}");
                Status::internal("Something went wrong")
            })?
        else {
            return Err(Status::invalid_argument(
                "Device with given public key doesn't exist",
            ));
        };
        BiometricAuth::validate_pubkey(&request.device_pub_key)?;
        let mobile_auth = BiometricAuth::new(device.id, request.auth_pub_key);
        let _ = mobile_auth.save(&self.pool).await.map_err(|err| {
            error!("Failed to save mobile auth into db: {err}");
            Status::internal("Failed to save results")
        })?;
        info!(
            "User {}({}) registered mobile auth for device {}({})",
            user.username, user.id, device.name, device.id
        );
        Ok(())
    }

    fn validate_activated_user(request: &ActivateUserRequest) -> Result<(), Status> {
        if let Some(ref phone_number) = request.phone_number {
            if !is_valid_phone_number(phone_number) {
                return Err(Status::new(
                    tonic::Code::InvalidArgument,
                    "invalid phone number",
                ));
            }
        }

        Ok(())
    }

    #[instrument(skip_all)]
    pub(crate) async fn activate_user(
        &self,
        request: ActivateUserRequest,
        req_device_info: Option<defguard_proto::proxy::DeviceInfo>,
    ) -> Result<(), Status> {
        debug!("Activating user account");
        let enrollment = self.validate_session(request.token.as_ref()).await?;
        Self::validate_activated_user(&request)?;

        let ip_address;
        let device_info;
        if let Some(ref info) = req_device_info {
            ip_address = info.ip_address.clone();
            let user_agent = info.user_agent.clone().unwrap_or_default();
            device_info = Some(get_device_info(&user_agent));
        } else {
            ip_address = String::new();
            device_info = None;
        }
        debug!("IP address {ip_address}, device info {device_info:?}");

        // check if password is strong enough
        debug!("Verifying password strength for user activation process.");
        if let Err(err) = check_password_strength(&request.password) {
            error!("Password not strong enough: {err}");
            return Err(Status::invalid_argument("password not strong enough"));
        }
        debug!("Password is strong enough to complete the user activation process.");

        // fetch related users
        let mut user = enrollment.fetch_user(&self.pool).await?;
        debug!(
            "Fetching user {} data to check if the user already has a password.",
            user.username
        );
        if user.has_password() {
            error!("User {} already activated", user.username);
            return Err(Status::invalid_argument("user already activated"));
        }
        debug!("User doesn't have a password yet. Continue user activation process...");

        debug!("Verify if the user is active or disabled.");
        if !user.is_active {
            warn!(
                "Can't finalize enrollment for disabled user {}",
                user.username
            );
            return Err(Status::invalid_argument("user is disabled"));
        }
        debug!("User is active.");

        let mut transaction = self.pool.begin().await.map_err(|err| {
            error!("Failed to begin transaction: {err}");
            Status::internal("unexpected error")
        })?;

        // update user
        info!("Update user details and set a new password.");
        user.phone = request.phone_number;
        user.set_password(&request.password);
        user.save(&mut *transaction).await.map_err(|err| {
            error!("Failed to update user {}: {err}", user.username);
            Status::internal("unexpected error")
        })?;
        debug!("Updating user details ended with success.");
        let _ = update_counts(&self.pool).await;

        debug!("Retriving settings to send welcome email...");
        let settings = Settings::get_current_settings();
        debug!("Settings successfully retrieved.");

        // send welcome email
        debug!("Try to send welcome email...");
        enrollment
            .send_welcome_email(
                &mut transaction,
                &self.mail_tx,
                &user,
                &settings,
                &ip_address,
                device_info.as_deref(),
            )
            .await?;

        // send success notification to admin
        debug!(
            "Trying to fetch admin data from the token to send notification about activating user."
        );
        let admin = enrollment.fetch_admin(&mut *transaction).await?;

        if let Some(admin) = admin {
            debug!("Send admin notification mail.");
            Token::send_admin_notification(
                &self.mail_tx,
                &admin,
                &user,
                &ip_address,
                device_info.as_deref(),
            )?;
        }

        // Unset the enrollment-pending flag (https://github.com/DefGuard/client/issues/647).
        user.enrollment_pending = false;
        user.save(&mut *transaction).await.map_err(|err| {
            error!(
                "Failed to unset enrollment_pending flag for user {}: {err}",
                user.username
            );
            Status::internal("unexpected error")
        })?;

        transaction.commit().await.map_err(|err| {
            error!("Failed to commit transaction: {err}");
            Status::internal("unexpected error")
        })?;

        ldap_add_user(&mut user, Some(&request.password), &self.pool).await;

        info!("User {} activated", user.username);

        // Prepare event context and push the event
        let (ip, user_agent) = parse_client_ip_agent(&req_device_info).map_err(Status::internal)?;
        let context = BidiRequestContext::new(user.id, user.username.clone(), ip, user_agent);
        self.emit_event(context, EnrollmentEvent::EnrollmentCompleted)
            .map_err(|err| {
                error!("Failed to send event. Reason: {err}",);
                Status::internal("unexpected error")
            })?;

        Ok(())
    }

    #[instrument(skip_all)]
    pub(crate) async fn create_device(
        &self,
        request: NewDevice,
        req_device_info: Option<defguard_proto::proxy::DeviceInfo>,
    ) -> Result<DeviceConfigResponse, Status> {
        debug!("Adding new user device");
        let enrollment_token = self.validate_session(request.token.as_ref()).await?;

        // fetch related users
        let user = enrollment_token.fetch_user(&self.pool).await?;

        // check if adding device by non-admin users is allowed
        debug!(
            "Fetching enterprise settings for device creation process for user {}({:?})",
            user.username, user.id,
        );
        let enterprise_settings = EnterpriseSettings::get(&self.pool).await.map_err(|err| {
            error!(
            "Failed to fetch enterprise settings for device creation process for user {}({:?}): \
            {err}",
            user.username, user.id,
        );
            Status::internal("unexpected error")
        })?;
        debug!("Enterprise settings: {enterprise_settings:?}");

        // add device
        debug!(
            "Verifying if user {}({:?}) is active",
            user.username, user.id
        );
        if !user.is_active {
            error!(
                "Can't create device for disabled user {}({:?})",
                user.username, user.id
            );
            return Err(Status::invalid_argument(
                "can't add device to disabled user",
            ));
        }
        info!(
            "User {}({:?}) is active, proceeding with device creation, pubkey: {}",
            user.username, user.id, request.pubkey
        );

        let ip_address;
        let device_info;
        if let Some(ref info) = req_device_info {
            ip_address = info.ip_address.clone();
            let user_agent = info.user_agent.clone().unwrap_or_default();
            device_info = Some(get_device_info(&user_agent));
        } else {
            ip_address = String::new();
            device_info = None;
        }
        debug!("IP address {}, device info {device_info:?}", ip_address);

        debug!(
            "Validating pubkey {} for device creation process for user {}({:?})",
            request.pubkey, user.username, user.id,
        );
        Device::validate_pubkey(&request.pubkey).map_err(|err| {
            error!(
                "Invalid pubkey {}, device won't be created for user {}({:?}): {err}",
                request.pubkey, user.username, user.id
            );
            Status::invalid_argument("invalid pubkey")
        })?;
        info!(
            "Pubkey {} is valid for device creation process for user {}({:?})",
            request.pubkey, user.username, user.id
        );

        // Make sure there is no device with the same pubkey, such state may lead to unexpected issues
        debug!(
            "Checking pubkey {} uniqueness for device creation process for user {}({:?}).",
            request.pubkey, user.username, user.id,
        );
        if let Some(device) = Device::find_by_pubkey(&self.pool, &request.pubkey)
            .await
            .map_err(|err| {
                error!(
                    "Failed to get device {} by its pubkey: {err}",
                    request.pubkey
                );
                Status::internal("unexpected error")
            })?
        {
            warn!(
                "User {}({:?}) failed to add device {}, identical pubkey ({}) already exists for device {}",
                user.username, user.id, request.name, request.pubkey, device.name
            );
            return Err(Status::invalid_argument("invalid key"));
        }
        info!(
            "Pubkey {} is unique for device creation process for user {}({:?}).",
            request.pubkey, user.username, user.id
        );

        let mut transaction = self.pool.begin().await.map_err(|err| {
            error!("Failed to begin transaction: {err}");
            Status::internal("unexpected error")
        })?;

        let (device, network_info, configs) = if let Some(device_id) = enrollment_token.device_id {
            debug!(
                "A device with ID {device_id} is attached to a received enrollment token, trying \
                to finish its configuration instead of creating a new one."
            );
            let mut device = Device::find_by_id(&mut *transaction, device_id)
                .await
                .map_err(|err| {
                    error!(
                        "Failed to find device with ID {device_id} for user {}({:?}): {err}",
                        user.username, user.id
                    );
                    Status::internal("unexpected error")
                })?
                .ok_or_else(|| {
                    error!(
                        "Device with ID {device_id} not found for user {}({:?}). Aborting device \
                        configuration process.",
                        user.username, user.id
                    );
                    Status::not_found("device not found")
                })?;

            // Currently not supported
            if device.device_type != DeviceType::Network {
                error!(
                    "Device {} added by user {}({:?}) is not a network device. Partial device \
                    configuration using a token is not supported for non-network devices.",
                    device.name, user.username, user.id
                );
                return Err(Status::invalid_argument("invalid device type"));
            }

            device.wireguard_pubkey.clone_from(&request.pubkey);
            device.configured = true;

            device.save(&mut *transaction).await.map_err(|err| {
                error!(
                    "Failed to save network device {} for user {}({:?}): {err}",
                    device.name, user.username, user.id
                );
                Status::internal("unexpected error")
            })?;

            let mut networks = device
                .find_network_device_networks(&mut *transaction)
                .await
                .map_err(|err| {
                    error!(
                        "Failed to find networks for device {} for user {}({:?}): {err}",
                        device.name, user.username, user.id
                    );
                    Status::internal("unexpected error")
                })?;

            let Some(network) = networks.pop() else {
                error!(
                    "Network device {} added by user {}({:?}) is not assigned to any networks. \
                    Aborting partial device configuration process.",
                    device.name, user.username, user.id
                );
                return Err(Status::not_found("network not found"));
            };
            // We popped the last network, there should be 0 left.
            if !networks.is_empty() {
                warn!(
                    "Network device {} added by user {}({:?}) is assigned to more than one \
                    network. Using the last network as a fallback.",
                    device.name, user.username, user.id
                );
            }

            let (network_info, configs) = device
                .get_network_configs(&network, &mut transaction)
                .await
                .map_err(|err| {
                    error!(
                        "Failed to get network configs for device {} for user {}({:?}): {err}",
                        device.name, user.username, user.id
                    );
                    Status::internal("unexpected error")
                })?;

            (device, vec![network_info], vec![configs])
        } else {
            debug!(
                "Creating new device for user {}({:?}): {}.",
                user.username, user.id, request.name
            );
            let device = Device::new(
                request.name.clone(),
                request.pubkey.clone(),
                enrollment_token.user_id,
                DeviceType::User,
                None,
                true,
            );
            if device.name.is_empty() {
                return Err(Status::invalid_argument(
                    "Cannot add a new device with no name. You may be trying to add a new user device as a network device. Defguard CLI supports only network devices.",
                ));
            }
            let device = device.save(&mut *transaction).await.map_err(|err| {
                error!(
                    "Failed to save device {}, pubkey {} for user {}({:?}): {err}",
                    request.name, request.pubkey, user.username, user.id,
                );
                Status::internal("unexpected error")
            })?;
            info!("New device created using a token: {device:?}.");
            let _ = update_counts(&self.pool).await;
            debug!(
                "Adding device {} to all existing user networks for user {}({:?}).",
                device.wireguard_pubkey, user.username, user.id,
            );
            let (network_info, configs) = device
                .add_to_all_networks(&mut transaction)
                .await
                .map_err(|err| {
                    error!(
                        "Failed to add device {} to existing networks: {err}",
                        device.name
                    );
                    Status::internal("unexpected error")
                })?;
            info!(
                "Added device {} to all existing user networks for user {}({:?})",
                device.wireguard_pubkey, user.username, user.id
            );
            (device, network_info, configs)
        };

        // get all locations affected by device being added
        let mut affected_location_ids = HashSet::new();
        for network_info_item in network_info.clone() {
            affected_location_ids.insert(network_info_item.network_id);
        }

        // send firewall config updates to affected locations
        // if they have ACL enabled & enterprise features are active
        for location_id in affected_location_ids {
            if let Some(location) = WireguardNetwork::find_by_id(&mut *transaction, location_id)
                .await
                .map_err(|err| {
                    error!("Failed to fetch WireguardNetwork with ID {location_id}: {err}",);
                    Status::internal("unexpected error")
                })?
            {
                if let Some(firewall_config) =
                    try_get_location_firewall_config(&location, &mut transaction)
                        .await
                        .map_err(|err| {
                            error!("Failed to get firewall config for location {location}: {err}",);
                            Status::internal("unexpected error")
                        })?
                {
                    debug!(
                        "Sending firewall config update for location {location} affected by adding new device {}, user {}({})",
                        device.wireguard_pubkey, user.username, user.id
                    );
                    self.send_wireguard_event(GatewayEvent::FirewallConfigChanged(
                        location_id,
                        firewall_config,
                    ));
                }
            }
        }

        debug!(
            "Sending DeviceCreated event to gateway for device {}, user {}({:?})",
            device.wireguard_pubkey, user.username, user.id,
        );
        self.send_wireguard_event(GatewayEvent::DeviceCreated(DeviceInfo {
            device: device.clone(),
            network_info,
        }));
        info!(
            "Sent DeviceCreated event to gateway for device {}, user {}({:?})",
            device.wireguard_pubkey, user.username, user.id,
        );

        debug!(
            "Fetching settings for device {} creation process for user {}({:?})",
            device.wireguard_pubkey, user.username, user.id,
        );
        let settings = Settings::get_current_settings();
        debug!("Settings: {settings:?}");

        // create polling token for further client communication
        debug!(
            "Creating polling token for further client communication for device {}, user {}({:?})",
            device.wireguard_pubkey, user.username, user.id,
        );
        let token = PollingToken::new(device.id)
            .save(&mut *transaction)
            .await
            .map_err(|err| {
                error!(
                    "Failed to save PollingToken for device {}, user {}({:?}): {err}",
                    device.wireguard_pubkey, user.username, user.id
                );
                Status::internal("failed to save polling token")
            })?;
        info!(
            "Created polling token for further client communication for device: {}, user {}({:?})",
            device.wireguard_pubkey, user.username, user.id,
        );

        transaction.commit().await.map_err(|err| {
            error!(
                "Failed to commit transaction, device {} won't be created for user {}({:?}): {err}",
                device.wireguard_pubkey, user.username, user.id,
            );
            Status::internal("unexpected error")
        })?;

        // Don't send them service locations if they don't support it
        let configs = configs
            .into_iter()
            .filter(|config| {
                config.service_location_mode == ServiceLocationMode::Disabled
                    || ClientFeature::ServiceLocations
                        .is_supported_by_device(req_device_info.as_ref())
            })
            .collect::<Vec<DeviceConfig>>();

        let template_locations: Vec<TemplateLocation> = configs
            .iter()
            .map(|c| TemplateLocation {
                name: c.network_name.clone(),
                assigned_ips: c.address.as_csv(),
            })
            .collect();

        debug!(
            "Sending device created mail for device {}, user {}({:?})",
            device.wireguard_pubkey, user.username, user.id
        );
        send_new_device_added_email(
            &device.name,
            &device.wireguard_pubkey,
            &template_locations,
            &user.email,
            &self.mail_tx,
            Some(&ip_address),
            device_info.as_deref(),
        )
        .map_err(|_| Status::internal("error rendering email template"))?;

        info!("Device {} remote configuration done.", device.name);

        let openid_provider = OpenIdProvider::get_current(&self.pool)
            .await
            .map_err(|err| {
                error!("Failed to get OpenID provider: {err}");
                Status::internal(format!("unexpected error: {err}"))
            })?;

        let instance_info = InstanceInfo::new(
            settings,
            &user.username,
            &enterprise_settings,
            openid_provider,
        )
        .map_err(|err| {
            error!("Failed to create instance info: {err}");
            Status::internal("unexpected error")
        })?;

        let response = DeviceConfigResponse {
            device: Some(device.clone().into()),
            configs: configs.into_iter().map(Into::into).collect(),
            instance: Some(instance_info.into()),
            token: Some(token.token),
        };

        // Prepare event context and push the event
        let (ip, user_agent) = parse_client_ip_agent(&req_device_info).map_err(Status::internal)?;
        let context = BidiRequestContext::new(user.id, user.username.clone(), ip, user_agent);
        self.emit_event(context, EnrollmentEvent::EnrollmentDeviceAdded { device })
            .map_err(|err| {
                error!("Failed to send event. Reason: {err}",);
                Status::internal("unexpected error")
            })?;

        Ok(response)
    }

    /// Get all information needed to update instance information for desktop client
    #[instrument(skip_all)]
    pub(crate) async fn get_network_info(
        &self,
        request: ExistingDevice,
        device_info: Option<defguard_proto::proxy::DeviceInfo>,
    ) -> Result<DeviceConfigResponse, Status> {
        debug!("Getting network info for device: {:?}", request.pubkey);
        let token = self.validate_session(request.token.as_ref()).await?;

        Device::validate_pubkey(&request.pubkey).map_err(|_| {
            error!("Invalid pubkey {}", &request.pubkey);
            Status::invalid_argument("invalid pubkey")
        })?;
        // Find existing device by public key.
        let Ok(Some(device)) = Device::find_by_pubkey(&self.pool, &request.pubkey).await else {
            error!("Failed to fetch device by pubkey: {}", &request.pubkey);
            return Err(Status::not_found("device not found"));
        };

        // check if device owner matches used enrollment token
        if device.user_id != token.user_id {
            error!(
                "Enrollment token does not match device with pubkey {}",
                request.pubkey
            );
            return Err(Status::unauthenticated(
                "enrollment token is not valid for specified device",
            ));
        }

        let token = new_polling_token(&self.pool, &device).await?;
        build_device_config_response(&self.pool, device, Some(token), device_info).await
    }

    // TODO: Add events
    #[instrument(skip_all)]
    pub(crate) async fn register_code_mfa_start(
        &self,
        request: CodeMfaSetupStartRequest,
    ) -> Result<CodeMfaSetupStartResponse, Status> {
        debug!("Begin enrollment code mfa setup start");
        let method = request.method();
        if method != MfaMethod::Email && method != MfaMethod::Totp {
            return Err(Status::invalid_argument("Method not supported".to_string()));
        }
        let enrollment = Token::find_by_id(&self.pool, &request.token).await?;
        let mut user = enrollment.fetch_user(&self.pool).await?;
        // available only for unenrolled users
        if user.is_enrolled() {
            return Err(Status::permission_denied("User is already enrolled"));
        }
        match method {
            MfaMethod::Email => {
                let settings = Settings::get_current_settings();
                if !settings.smtp_configured() {
                    error!("Unable to start Email mfa setup. SMTP is not configured");
                    return Err(Status::internal("SMTP not configured".to_string()));
                }
                if user.email_mfa_enabled {
                    return Err(Status::invalid_argument(
                        "Method already enabled".to_string(),
                    ));
                }
                user.new_email_secret(&self.pool).await.map_err(|_| {
                    error!("Failed to create email secret");
                    Status::internal("Failed to setup email mfa".to_string())
                })?;
                info!("Created email secret for {}", &user.username);
                send_email_mfa_activation_email(&user, &self.mail_tx, None).map_err(|e| {
                    error!("Failed to send email mfa activation email.\nReason:{e}");
                    Status::internal("Failed to send activation email".to_string())
                })?;
                Ok(CodeMfaSetupStartResponse { totp_secret: None })
            }
            MfaMethod::Totp => {
                if user.totp_enabled {
                    return Err(Status::invalid_argument(
                        "Method already enabled".to_string(),
                    ));
                }
                let secret = user.new_totp_secret(&self.pool).await.map_err(|_| {
                    error!("Failed to make new totp secret");
                    Status::internal(String::new())
                })?;
                info!("New totp secret created for {}", &user.username);
                Ok(CodeMfaSetupStartResponse {
                    totp_secret: Some(secret),
                })
            }
            _ => Err(Status::invalid_argument("Method not supported".to_string())),
        }
    }

    // TODO: Add events
    #[instrument(skip_all)]
    pub(crate) async fn register_code_mfa_finish(
        &self,
        request: CodeMfaSetupFinishRequest,
    ) -> Result<CodeMfaSetupFinishResponse, Status> {
        debug!("Begin enrollment code mfa setup finish");
        let enrollment = self.validate_session(Some(&request.token)).await?;
        let method = request.method();
        if method != MfaMethod::Totp && method != MfaMethod::Email {
            return Err(Status::invalid_argument("Method not supported"));
        }
        let mut user = enrollment.fetch_user(&self.pool).await?;
        if user.mfa_enabled {
            return Err(Status::invalid_argument(
                "Mfa already enabled on the account".to_string(),
            ));
        }
        // available only for unenrolled users
        if user.is_enrolled() {
            return Err(Status::permission_denied("User is already enrolled"));
        }
        let mfa_method: MFAMethod;
        // enable corresponding MFA
        match method {
            MfaMethod::Email => {
                if !user.verify_email_mfa_code(&request.code) {
                    return Err(Status::invalid_argument("Email code invalid".to_string()));
                }
                user.enable_email_mfa(&self.pool)
                    .await
                    .map_err(|_| Status::internal("Enabling method failed.".to_string()))?;
                mfa_method = MFAMethod::Email;
            }
            MfaMethod::Totp => {
                if !user.verify_totp_code(&request.code) {
                    return Err(Status::invalid_argument("Code invalid".to_string()));
                }
                user.enable_totp(&self.pool)
                    .await
                    .map_err(|_| Status::internal("Enabling method failed.".to_string()))?;
                mfa_method = MFAMethod::OneTimePassword;
            }
            _ => {
                return Err(Status::invalid_argument("Method not supported"));
            }
        }
        user.enable_mfa(&self.pool)
            .await
            .map_err(|_| Status::internal("Enabling MFA on the account failed.".to_string()))?;
        let recovery_codes = user
            .get_recovery_codes(&self.pool)
            .await
            .map_err(|_| Status::internal("Failed to get recovery codes.".to_string()))?
            .ok_or_else(|| Status::internal("Recovery codes not found".to_string()))?;
        if let Err(e) = send_mfa_configured_email(None, &user, &mfa_method, &self.mail_tx) {
            error!("Failed to send mfa configured email\nReason: {e}");
        }
        info!(
            "Successfully enabled MFA method {} for user {}",
            method.as_str_name(),
            &user.username
        );
        Ok(CodeMfaSetupFinishResponse { recovery_codes })
    }
}

async fn initial_info_from_user(
    pool: &PgPool,
    user: User<Id>,
) -> Result<InitialUserInfo, sqlx::Error> {
    let enrolled = user.is_enrolled();
    let devices = user.user_devices(pool).await?;
    let device_names = devices.into_iter().map(|dev| dev.device.name).collect();
    let is_admin = user.is_admin(pool).await?;
    Ok(InitialUserInfo {
        first_name: user.first_name,
        last_name: user.last_name,
        login: user.username,
        email: user.email,
        phone_number: user.phone,
        is_active: user.is_active,
        device_names,
        enrolled,
        is_admin,
    })
}

#[cfg(test)]
mod test {
    use defguard_common::{
        config::{DefGuardConfig, SERVER_CONFIG},
        db::{
            models::{
                Settings, User,
                settings::{defaults::WELCOME_EMAIL_SUBJECT, initialize_current_settings},
            },
            setup_pool,
        },
    };
    use defguard_core::db::models::enrollment::{ENROLLMENT_TOKEN_TYPE, Token};
    use defguard_mail::Mail;
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use tokio::sync::mpsc::unbounded_channel;

    #[sqlx::test]
    async fn dg25_11_test_enrollment_welcome_email(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        // initialize server config
        SERVER_CONFIG
            .set(DefGuardConfig::new_test_config())
            .unwrap();

        // setup mail channel
        let (mail_tx, mut mail_rx) = unbounded_channel::<Mail>();

        // setup users
        let admin = User::new(
            "test_admin",
            Some("pass123"),
            "Test",
            "Admin",
            "admin@test.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();
        let user = User::new(
            "test_user",
            Some("pass123"),
            "Test",
            "User",
            "user@test.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        // generate enrollment token
        let token = Token::new(
            user.id,
            Some(admin.id),
            Some(user.email.clone()),
            10,
            Some(ENROLLMENT_TOKEN_TYPE.to_string()),
        );

        // initialize settings
        Settings::init_defaults(&pool).await.unwrap();
        initialize_current_settings(&pool).await.unwrap();

        let mut settings = Settings::get(&pool).await.unwrap().unwrap();

        // send welcome email
        let mut transaction = pool.begin().await.unwrap();
        token
            .send_welcome_email(
                &mut transaction,
                &mail_tx,
                &user,
                &settings,
                "127.0.0.1",
                None,
            )
            .await
            .unwrap();

        // check email content
        let mail = mail_rx.recv().await.unwrap();
        assert_eq!(mail.to, user.email);
        assert_eq!(
            mail.subject,
            settings.enrollment_welcome_email_subject.unwrap()
        );

        // set subject to None
        settings.enrollment_welcome_email_subject = None;

        // send another welcome email
        let mut transaction = pool.begin().await.unwrap();
        token
            .send_welcome_email(
                &mut transaction,
                &mail_tx,
                &user,
                &settings,
                "127.0.0.1",
                None,
            )
            .await
            .unwrap();

        // check email content
        let mail = mail_rx.recv().await.unwrap();
        assert_eq!(mail.to, user.email);
        assert_eq!(mail.subject, WELCOME_EMAIL_SUBJECT);
    }
}
