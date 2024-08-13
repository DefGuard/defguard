use std::sync::Arc;

use ipnetwork::IpNetwork;
use reqwest::Url;
use sqlx::Transaction;
use tokio::sync::{broadcast::Sender, mpsc::UnboundedSender};
use tonic::Status;
use uaparser::UserAgentParser;

use super::proto::{
    ActivateUserRequest, AdminInfo, Device as ProtoDevice, DeviceConfig as ProtoDeviceConfig,
    DeviceConfigResponse, EnrollmentStartRequest, EnrollmentStartResponse, ExistingDevice,
    InitialUserInfo, NewDevice,
};
use crate::{
    db::{
        models::{
            device::{DeviceConfig, DeviceInfo, WireguardNetworkDevice},
            enrollment::{Token, TokenError, ENROLLMENT_TOKEN_TYPE},
            wireguard::WireguardNetwork,
        },
        DbPool, Device, GatewayEvent, Settings, User,
    },
    handlers::{mail::send_new_device_added_email, user::check_password_strength},
    headers::get_device_info,
    ldap::utils::ldap_add_user,
    mail::Mail,
    server_config,
    templates::{self, TemplateLocation},
};

pub(super) struct EnrollmentServer {
    pool: DbPool,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
    user_agent_parser: Arc<UserAgentParser>,
    ldap_feature_active: bool,
}

struct InstanceInfo {
    id: uuid::Uuid,
    name: String,
    url: Url,
    proxy_url: Url,
    username: String,
}

impl InstanceInfo {
    pub fn new<S: Into<String>>(settings: Settings, username: S) -> Self {
        let config = server_config();
        InstanceInfo {
            id: settings.uuid,
            name: settings.instance_name,
            url: config.url.clone(),
            proxy_url: config.enrollment_url.clone(),
            username: username.into(),
        }
    }
}

impl From<InstanceInfo> for super::proto::InstanceInfo {
    fn from(instance: InstanceInfo) -> Self {
        Self {
            name: instance.name,
            id: instance.id.to_string(),
            url: instance.url.to_string(),
            proxy_url: instance.proxy_url.to_string(),
            username: instance.username,
        }
    }
}

impl EnrollmentServer {
    #[must_use]
    pub fn new(
        pool: DbPool,
        wireguard_tx: Sender<GatewayEvent>,
        mail_tx: UnboundedSender<Mail>,
        user_agent_parser: Arc<UserAgentParser>,
    ) -> Self {
        // FIXME: check if LDAP feature is enabled
        let ldap_feature_active = true;
        Self {
            pool,
            wireguard_tx,
            mail_tx,
            user_agent_parser,
            ldap_feature_active,
        }
    }

    // check if token provided with request corresponds to a valid enrollment session
    async fn validate_session(&self, token: Option<&str>) -> Result<Token, Status> {
        let Some(token) = token else {
            error!("Missing authorization header in request");
            return Err(Status::unauthenticated("Missing authorization header"));
        };
        debug!("Validating enrollment session token: {token}");

        let enrollment = Token::find_by_id(&self.pool, token).await?;
        if enrollment.is_session_valid(server_config().enrollment_session_timeout.as_secs()) {
            info!("Enrollment session validated");
            Ok(enrollment)
        } else {
            error!("Enrollment session expired");
            Err(Status::unauthenticated("Enrollment session expired"))
        }
    }

    /// Sends given `GatewayEvent` to be handled by gateway GRPC server
    pub fn send_wireguard_event(&self, event: GatewayEvent) {
        if let Err(err) = self.wireguard_tx.send(event) {
            error!("Error sending WireGuard event {err}");
        }
    }

    pub async fn start_enrollment(
        &self,
        request: EnrollmentStartRequest,
    ) -> Result<EnrollmentStartResponse, Status> {
        debug!("Starting enrollment session, request: {request:?}");
        // fetch enrollment token
        let mut enrollment = Token::find_by_id(&self.pool, &request.token).await?;

        if let Some(token_type) = &enrollment.token_type {
            if token_type != ENROLLMENT_TOKEN_TYPE {
                error!("Invalid token type used while trying to start enrollment: {token_type}");
                return Err(Status::permission_denied("invalid token"));
            }

            // fetch related users
            let user = enrollment.fetch_user(&self.pool).await?;
            let admin = enrollment.fetch_admin(&self.pool).await?;

            if !user.is_active {
                warn!("Can't start enrollment for disabled user {}", user.username);
                return Err(Status::permission_denied("user is disabled"));
            };

            let mut transaction = self.pool.begin().await.map_err(|_| {
                error!("Failed to begin transaction");
                Status::internal("unexpected error")
            })?;

            // validate token & start session
            debug!("Starting enrollment session for user {}", user.username);
            let session_deadline = enrollment
                .start_session(
                    &mut transaction,
                    server_config().enrollment_session_timeout.as_secs(),
                )
                .await?;
            info!("Enrollment session started for user {}", user.username);

            let settings = Settings::get_settings(&mut *transaction)
                .await
                .map_err(|_| {
                    error!("Failed to get settings");
                    Status::internal("unexpected error")
                })?;

            let vpn_setup_optional = settings.enrollment_vpn_step_optional;
            let instance_info = InstanceInfo::new(settings, &user.username);

            let user_info = InitialUserInfo::from_user(&self.pool, user)
                .await
                .map_err(|_| {
                    error!("Failed to get user info");
                    Status::internal("unexpected error")
                })?;

            let admin_info = admin.map(AdminInfo::from);

            let response = super::proto::EnrollmentStartResponse {
                admin: admin_info,
                user: Some(user_info),
                deadline_timestamp: session_deadline.and_utc().timestamp(),
                final_page_content: enrollment
                    .get_welcome_page_content(&mut transaction)
                    .await?,
                vpn_setup_optional,
                instance: Some(instance_info.into()),
            };

            transaction.commit().await.map_err(|_| {
                error!("Failed to commit transaction");
                Status::internal("unexpected error")
            })?;

            Ok(response)
        } else {
            Err(Status::permission_denied("invalid token"))
        }
    }

    pub async fn activate_user(
        &self,
        request: ActivateUserRequest,
        req_device_info: Option<super::proto::DeviceInfo>,
    ) -> Result<(), Status> {
        debug!("Activating user account: {request:?}");
        let enrollment = self.validate_session(request.token.as_deref()).await?;

        let ip_address;
        let device_info;
        if let Some(info) = req_device_info {
            ip_address = info.ip_address.unwrap_or_default();
            let user_agent = info.user_agent.unwrap_or_default();
            device_info = get_device_info(&self.user_agent_parser, &user_agent);
        } else {
            ip_address = String::new();
            device_info = None;
        }

        // check if password is strong enough
        if let Err(err) = check_password_strength(&request.password) {
            error!("Password not strong enough: {err}");
            return Err(Status::invalid_argument("password not strong enough"));
        }

        // fetch related users
        let mut user = enrollment.fetch_user(&self.pool).await?;
        if user.has_password() {
            error!("User {} already activated", user.username);
            return Err(Status::invalid_argument("user already activated"));
        }

        if !user.is_active {
            warn!(
                "Can't finalize enrollment for disabled user {}",
                user.username
            );
            return Err(Status::invalid_argument("user is disabled"));
        }

        let mut transaction = self.pool.begin().await.map_err(|_| {
            error!("Failed to begin transaction");
            Status::internal("unexpected error")
        })?;

        // update user
        user.phone = request.phone_number;
        user.set_password(&request.password);
        user.save(&mut *transaction).await.map_err(|err| {
            error!("Failed to update user {}: {err}", user.username);
            Status::internal("unexpected error")
        })?;

        // sync with LDAP
        if self.ldap_feature_active {
            let _result = ldap_add_user(&self.pool, &user, &request.password).await;
        };

        let settings = Settings::get_settings(&mut *transaction)
            .await
            .map_err(|_| {
                error!("Failed to get settings");
                Status::internal("unexpected error")
            })?;

        // send welcome email
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
        let admin = enrollment.fetch_admin(&mut *transaction).await?;

        if let Some(admin) = admin {
            Token::send_admin_notification(
                &self.mail_tx,
                &admin,
                &user,
                &ip_address,
                device_info.as_deref(),
            )?;
        }

        transaction.commit().await.map_err(|_| {
            error!("Failed to commit transaction");
            Status::internal("unexpected error")
        })?;

        info!("User {} activated", user.username);

        Ok(())
    }

    pub async fn create_device(
        &self,
        request: NewDevice,
        req_device_info: Option<super::proto::DeviceInfo>,
    ) -> Result<DeviceConfigResponse, Status> {
        debug!("Adding new user device: {request:?}");
        let enrollment = self.validate_session(request.token.as_deref()).await?;

        // fetch related users
        let user = enrollment.fetch_user(&self.pool).await?;

        // add device
        if !user.is_active {
            error!("Can't create device for a disabled user {}", user.username);
            return Err(Status::invalid_argument(
                "can't add device to disabled user",
            ));
        }

        let ip_address;
        let device_info;
        if let Some(info) = req_device_info {
            ip_address = info.ip_address.unwrap_or_default();
            let user_agent = info.user_agent.unwrap_or_default();
            device_info = get_device_info(&self.user_agent_parser, &user_agent);
        } else {
            ip_address = String::new();
            device_info = None;
        }

        Device::validate_pubkey(&request.pubkey).map_err(|_| {
            error!("Invalid pubkey {}", request.pubkey);
            Status::invalid_argument("invalid pubkey")
        })?;

        // Make sure there is no device with the same pubkey, such state may lead to unexpected issues
        if let Some(device) = Device::find_by_pubkey(&self.pool, &request.pubkey)
            .await
            .map_err(|_| {
                error!("Failed to get device by its pubkey: {}", request.pubkey);
                Status::internal("unexpected error")
            })?
        {
            warn!(
                "User {} failed to add device {}, identical pubkey ({}) already exists for device {}",
                user.username,
                request.name,
                request.pubkey,
                device.name
            );
            return Err(Status::invalid_argument("invalid key"));
        };

        let mut device = Device::new(request.name, request.pubkey, enrollment.user_id);

        let mut transaction = self.pool.begin().await.map_err(|_| {
            error!("Failed to begin transaction");
            Status::internal("unexpected error")
        })?;
        device.save(&mut *transaction).await.map_err(|err| {
            error!("Failed to save device {}: {err}", device.name);
            Status::internal("unexpected error")
        })?;

        let (network_info, configs) =
            device
                .add_to_all_networks(&mut transaction)
                .await
                .map_err(|err| {
                    error!(
                        "Failed to add device {} to existing networks: {err}",
                        device.name
                    );
                    Status::internal("unexpected error")
                })?;

        self.send_wireguard_event(GatewayEvent::DeviceCreated(DeviceInfo {
            device: device.clone(),
            network_info,
        }));

        let settings = Settings::get_settings(&mut *transaction)
            .await
            .map_err(|_| {
                error!("Failed to get settings");
                Status::internal("unexpected error")
            })?;

        transaction.commit().await.map_err(|_| {
            error!("Failed to commit transaction");
            Status::internal("unexpected error")
        })?;

        let template_locations: Vec<TemplateLocation> = configs
            .iter()
            .map(|c| TemplateLocation {
                name: c.network_name.clone(),
                assigned_ip: c.address.to_string(),
            })
            .collect();

        send_new_device_added_email(
            &device.name,
            &device.wireguard_pubkey,
            &template_locations,
            &user.email,
            &self.mail_tx,
            Some(&ip_address),
            device_info.as_deref(),
        )
        .map_err(|_| Status::internal("Failed to render new device added template"))?;

        info!(
            "Device {} assigned to user {} and added to all networks.",
            device.name, user.username
        );

        let response = DeviceConfigResponse {
            device: Some(device.into()),
            configs: configs.into_iter().map(Into::into).collect(),
            instance: Some(InstanceInfo::new(settings, &user.username).into()),
        };

        Ok(response)
    }

    /// Get all information needed
    /// to update instance information for desktop client
    pub async fn get_network_info(
        &self,
        request: ExistingDevice,
    ) -> Result<DeviceConfigResponse, Status> {
        debug!("Getting network info for device: {:?}", request.pubkey);
        let enrollment = self.validate_session(request.token.as_deref()).await?;

        // get enrollment user
        let user = enrollment.fetch_user(&self.pool).await?;

        Device::validate_pubkey(&request.pubkey).map_err(|_| {
            error!("Invalid pubkey {}", request.pubkey);
            Status::invalid_argument("invalid pubkey")
        })?;
        // Find existing device by public key
        let device = Device::find_by_pubkey(&self.pool, &request.pubkey)
            .await
            .map_err(|_| {
                error!("Failed to get device by its pubkey: {}", request.pubkey);
                Status::internal("unexpected error")
            })?;

        let settings = Settings::get_settings(&self.pool).await.map_err(|_| {
            error!("Failed to get settings");
            Status::internal("unexpected error")
        })?;

        let networks = WireguardNetwork::all(&self.pool).await.map_err(|err| {
            error!("Failed to fetch all networks: {err}");
            Status::internal(format!("unexpected error: {err}"))
        })?;

        let mut configs: Vec<ProtoDeviceConfig> = Vec::new();
        if let Some(device) = device {
            for network in networks {
                let (Some(device_id), Some(network_id)) = (device.id, network.id) else {
                    continue;
                };
                let wireguard_network_device =
                    WireguardNetworkDevice::find(&self.pool, device_id, network_id)
                        .await
                        .map_err(|err| {
                            error!("Failed to fetch wireguard network device for device {} and network {}: {err}", device_id, network_id);
                            Status::internal(format!("unexpected error: {err}"))
                        })?;
                if let Some(wireguard_network_device) = wireguard_network_device {
                    let allowed_ips = network
                        .allowed_ips
                        .iter()
                        .map(IpNetwork::to_string)
                        .collect::<Vec<String>>()
                        .join(",");
                    let config = ProtoDeviceConfig {
                        config: device.create_config(&network, &wireguard_network_device),
                        network_id,
                        network_name: network.name,
                        assigned_ip: wireguard_network_device.wireguard_ip.to_string(),
                        endpoint: format!("{}:{}", network.endpoint, network.port),
                        pubkey: network.pubkey,
                        allowed_ips,
                        dns: network.dns,
                        mfa_enabled: network.mfa_enabled,
                        keepalive_interval: network.keepalive_interval,
                    };
                    configs.push(config);
                }
            }

            info!("Device {} configs fetched", device.name);

            let response = DeviceConfigResponse {
                device: Some(device.into()),
                configs,
                instance: Some(InstanceInfo::new(settings, &user.username).into()),
            };

            Ok(response)
        } else {
            Err(Status::internal("device not found error"))
        }
    }
}

impl From<User> for AdminInfo {
    fn from(admin: User) -> Self {
        Self {
            name: format!("{} {}", admin.first_name, admin.last_name),
            phone_number: admin.phone,
            email: admin.email,
        }
    }
}

impl InitialUserInfo {
    async fn from_user(pool: &DbPool, user: User) -> Result<Self, sqlx::Error> {
        let enrolled = user.is_enrolled();
        let devices = user.devices(pool).await?;
        let device_names = devices.into_iter().map(|dev| dev.device.name).collect();
        Ok(Self {
            first_name: user.first_name,
            last_name: user.last_name,
            login: user.username,
            email: user.email,
            phone_number: user.phone,
            is_active: user.is_active,
            device_names,
            enrolled,
        })
    }
}

impl From<DeviceConfig> for ProtoDeviceConfig {
    fn from(config: DeviceConfig) -> Self {
        let allowed_ips = config
            .allowed_ips
            .iter()
            .map(IpNetwork::to_string)
            .collect::<Vec<String>>()
            .join(",");
        Self {
            network_id: config.network_id,
            network_name: config.network_name,
            config: config.config,
            endpoint: config.endpoint,
            assigned_ip: config.address.to_string(),
            pubkey: config.pubkey,
            allowed_ips,
            dns: config.dns,
            mfa_enabled: config.mfa_enabled,
            keepalive_interval: config.keepalive_interval,
        }
    }
}

impl From<Device> for ProtoDevice {
    fn from(device: Device) -> Self {
        Self {
            id: device.get_id().expect("Failed to get device ID"),
            name: device.name,
            pubkey: device.wireguard_pubkey,
            user_id: device.user_id,
            created_at: device.created.and_utc().timestamp(),
        }
    }
}

impl Token {
    // Send configured welcome email to user after finishing enrollment
    async fn send_welcome_email(
        &self,
        transaction: &mut Transaction<'_, sqlx::Postgres>,
        mail_tx: &UnboundedSender<Mail>,
        user: &User,
        settings: &Settings,
        ip_address: &str,
        device_info: Option<&str>,
    ) -> Result<(), TokenError> {
        debug!("Sending welcome mail to {}", user.username);
        let mail = Mail {
            to: user.email.clone(),
            subject: settings.enrollment_welcome_email_subject.clone().unwrap(),
            content: self
                .get_welcome_email_content(&mut *transaction, ip_address, device_info)
                .await?,
            attachments: Vec::new(),
            result_tx: None,
        };
        match mail_tx.send(mail) {
            Ok(()) => {
                info!("Sent enrollment welcome mail to {}", user.username);
                Ok(())
            }
            Err(err) => {
                error!("Error sending welcome mail: {err}");
                Err(TokenError::NotificationError(err.to_string()))
            }
        }
    }

    // Notify admin that a user has completed enrollment
    fn send_admin_notification(
        mail_tx: &UnboundedSender<Mail>,
        admin: &User,
        user: &User,
        ip_address: &str,
        device_info: Option<&str>,
    ) -> Result<(), TokenError> {
        debug!(
            "Sending enrollment success notification for user {} to {}",
            user.username, admin.username
        );
        let mail = Mail {
            to: admin.email.clone(),
            subject: "[defguard] User enrollment completed".into(),
            content: templates::enrollment_admin_notification(
                user,
                admin,
                ip_address,
                device_info,
            )?,
            attachments: Vec::new(),
            result_tx: None,
        };
        match mail_tx.send(mail) {
            Ok(()) => {
                info!(
                    "Sent enrollment success notification for user {} to {}",
                    user.username, admin.username
                );
                Ok(())
            }
            Err(err) => {
                error!("Error sending welcome mail: {err}");
                Err(TokenError::NotificationError(err.to_string()))
            }
        }
    }
}
