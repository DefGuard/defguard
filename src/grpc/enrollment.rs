use std::sync::Arc;

use crate::{
    config::DefGuardConfig,
    db::{
        models::{
            device::{DeviceConfig, DeviceInfo, WireguardNetworkDevice},
            enrollment::{Enrollment, EnrollmentError},
            wireguard::WireguardNetwork,
        },
        DbPool, Device, GatewayEvent, Settings, User,
    },
    handlers::{mail::send_new_device_added_email, user::check_password_strength},
    headers::get_device_info,
    ldap::utils::ldap_add_user,
    mail::Mail,
    templates::{self, TemplateLocation},
};
use ipnetwork::IpNetwork;
use reqwest::Url;
use sqlx::Transaction;
use tokio::sync::{broadcast::Sender, mpsc::UnboundedSender};
use tonic::{Request, Response, Status};

#[allow(non_snake_case)]
pub mod proto {
    tonic::include_proto!("enrollment");
}
use proto::{
    enrollment_service_server, ActivateUserRequest, AdminInfo, Device as ProtoDevice,
    DeviceConfig as ProtoDeviceConfig, DeviceConfigResponse, EnrollmentStartRequest,
    EnrollmentStartResponse, ExistingDevice, InitialUserInfo, NewDevice,
};
use uaparser::UserAgentParser;

pub struct EnrollmentServer {
    pool: DbPool,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
    user_agent_parser: Arc<UserAgentParser>,
    config: DefGuardConfig,
    ldap_feature_active: bool,
}

struct Instance {
    id: uuid::Uuid,
    name: String,
    url: Url,
}

impl Instance {
    pub fn new(settings: Settings, url: Url) -> Self {
        Instance {
            id: settings.uuid,
            name: settings.instance_name,
            url,
        }
    }
}

impl From<Instance> for proto::InstanceInfo {
    fn from(instance: Instance) -> Self {
        Self {
            name: instance.name,
            id: instance.id.to_string(),
            url: instance.url.to_string(),
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
        config: DefGuardConfig,
    ) -> Self {
        // FIXME: check if LDAP feature is enabled
        let ldap_feature_active = true;
        Self {
            pool,
            wireguard_tx,
            mail_tx,
            user_agent_parser,
            config,
            ldap_feature_active,
        }
    }

    // check if token provided with request corresponds to a valid enrollment session
    async fn validate_session<T: std::fmt::Debug>(
        &self,
        request: &Request<T>,
    ) -> Result<Enrollment, Status> {
        debug!("Validating enrollment session token: {request:?}");
        let token = if let Some(token) = request.metadata().get("authorization") {
            token
                .to_str()
                .map_err(|_| Status::unauthenticated("Invalid token"))?
        } else {
            error!("Missing authorization header in request");
            return Err(Status::unauthenticated("Missing authorization header"));
        };

        let enrollment = Enrollment::find_by_id(&self.pool, token).await?;

        if enrollment.is_session_valid(self.config.enrollment_session_timeout.as_secs()) {
            Ok(enrollment)
        } else {
            error!("Enrollment session expired");
            Err(Status::unauthenticated("Enrollment session expired"))
        }
    }

    /// Sends given `GatewayEvent` to be handled by gateway GRPC server
    pub fn send_wireguard_event(&self, event: GatewayEvent) {
        if let Err(err) = self.wireguard_tx.send(event) {
            error!("Error sending wireguard event {err}");
        }
    }
}

#[tonic::async_trait]
impl enrollment_service_server::EnrollmentService for EnrollmentServer {
    async fn start_enrollment(
        &self,
        request: Request<EnrollmentStartRequest>,
    ) -> Result<Response<EnrollmentStartResponse>, Status> {
        debug!("Starting enrollment session: {request:?}");
        let request = request.into_inner();
        // fetch enrollment token
        let mut enrollment = Enrollment::find_by_id(&self.pool, &request.token).await?;

        // fetch related users
        let user = enrollment.fetch_user(&self.pool).await?;
        let admin = enrollment.fetch_admin(&self.pool).await?;

        let mut transaction = self.pool.begin().await.map_err(|_| {
            error!("Failed to begin transaction");
            Status::internal("unexpected error")
        })?;

        // validate token & start session
        info!("Starting enrollment session for user {}", user.username);
        let session_deadline = enrollment
            .start_session(
                &mut transaction,
                self.config.enrollment_session_timeout.as_secs(),
            )
            .await?;

        let settings = Settings::get_settings(&mut *transaction)
            .await
            .map_err(|_| {
                error!("Failed to get settings");
                Status::internal("unexpected error")
            })?;

        let response = EnrollmentStartResponse {
            admin: Some(admin.into()),
            user: Some(user.into()),
            deadline_timestamp: session_deadline.timestamp(),
            final_page_content: enrollment
                .get_welcome_page_content(&mut transaction)
                .await?,
            vpn_setup_optional: settings.enrollment_vpn_step_optional,
            instance: Some(Instance::new(settings, self.config.url.clone()).into()),
        };

        transaction.commit().await.map_err(|_| {
            error!("Failed to commit transaction");
            Status::internal("unexpected error")
        })?;

        Ok(Response::new(response))
    }

    async fn activate_user(
        &self,
        request: Request<ActivateUserRequest>,
    ) -> Result<Response<()>, Status> {
        debug!("Activating user account: {request:?}");
        let enrollment = self.validate_session(&request).await?;

        let ip_address = request
            .metadata()
            .get("ip_address")
            .and_then(|value| value.to_str().map(ToString::to_string).ok())
            .unwrap_or_default();

        let user_agent = request
            .metadata()
            .get("user_agent")
            .and_then(|value| value.to_str().map(ToString::to_string).ok())
            .unwrap_or_default();

        let device_info = get_device_info(&self.user_agent_parser, &user_agent);

        // check if password is strong enough
        let request = request.into_inner();
        if let Err(err) = check_password_strength(&request.password) {
            error!("Password not strong enough: {err}");
            return Err(Status::invalid_argument("password not strong enough"));
        }

        // fetch related users
        let mut user = enrollment.fetch_user(&self.pool).await?;
        info!("Activating user account for {}", user.username);
        if user.has_password() {
            error!("User {} already activated", user.username);
            return Err(Status::invalid_argument("user already activated"));
        }

        let mut transaction = self.pool.begin().await.map_err(|_| {
            error!("Failed to begin transaction");
            Status::internal("unexpected error")
        })?;

        // update user
        user.phone = Some(request.phone_number);
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
                ip_address.clone(),
                device_info.clone(),
            )
            .await?;

        // send success notification to admin
        let admin = enrollment.fetch_admin(&mut *transaction).await?;
        Enrollment::send_admin_notification(&self.mail_tx, &admin, &user, ip_address, device_info)?;

        transaction.commit().await.map_err(|_| {
            error!("Failed to commit transaction");
            Status::internal("unexpected error")
        })?;

        Ok(Response::new(()))
    }

    async fn create_device(
        &self,
        request: Request<NewDevice>,
    ) -> Result<Response<DeviceConfigResponse>, Status> {
        debug!("Adding new user device: {request:?}");
        let enrollment = self.validate_session(&request).await?;

        // fetch related users
        let user = enrollment.fetch_user(&self.pool).await?;

        // add device
        info!("Adding new device for user {}", user.username);

        let ip_address = request
            .metadata()
            .get("ip_address")
            .and_then(|value| value.to_str().map(ToString::to_string).ok())
            .unwrap_or_default();

        let user_agent = request
            .metadata()
            .get("user_agent")
            .and_then(|value| value.to_str().map(ToString::to_string).ok())
            .unwrap_or_default();

        let device_info = get_device_info(&self.user_agent_parser, &user_agent);

        let request = request.into_inner();
        Device::validate_pubkey(&request.pubkey).map_err(|_| {
            error!("Invalid pubkey {}", request.pubkey);
            Status::invalid_argument("invalid pubkey")
        })?;
        let mut device = Device::new(request.name, request.pubkey, enrollment.user_id);

        let mut transaction = self.pool.begin().await.map_err(|_| {
            error!("Failed to begin transaction");
            Status::internal("unexpected error")
        })?;
        device.save(&mut *transaction).await.map_err(|err| {
            error!("Failed to save device {}: {err}", device.name);
            Status::internal("unexpected error")
        })?;

        let (network_info, configs) = device
            .add_to_all_networks(&mut transaction, &self.config.admin_groupname)
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
            ip_address,
            device_info,
        )
        .await
        .map_err(|_| Status::internal("Failed to render new device added tempalte"))?;
        let response = DeviceConfigResponse {
            device: Some(device.into()),
            configs: configs.into_iter().map(Into::into).collect(),
            instance: Some(Instance::new(settings, self.config.url.clone()).into()),
        };

        Ok(Response::new(response))
    }

    /// Get all information needed
    /// to update instance information for desktop client
    async fn get_network_info(
        &self,
        request: Request<ExistingDevice>,
    ) -> Result<Response<DeviceConfigResponse>, Status> {
        let _enrollment = self.validate_session(&request).await?;

        let request = request.into_inner();
        Device::validate_pubkey(&request.pubkey).map_err(|_| {
            error!("Invalid pubkey {}", request.pubkey);
            Status::invalid_argument("invalid pubkey")
        })?;
        // Find existing device by public key
        let device = Device::find_by_pubkey(&self.pool, &request.pubkey)
            .await
            .map_err(|_| {
                error!("Failed to get device");
                Status::internal("unexpected error")
            })?;

        let settings = Settings::get_settings(&self.pool).await.map_err(|_| {
            error!("Failed to get settings");
            Status::internal("unexpected error")
        })?;

        let networks = WireguardNetwork::all(&self.pool).await.map_err(|err| {
            error!("Invalid failed to get networks {err}");
            Status::internal(format!("unexpected error: {err}"))
        })?;

        let mut configs: Vec<ProtoDeviceConfig> = vec![];
        if let Some(device) = device {
            for network in networks {
                let wireguard_network_device = WireguardNetworkDevice::find(
                    &self.pool,
                    device.id.unwrap(),
                    network.id.unwrap(),
                )
                .await
                .map_err(|err| {
                    error!("Invalid failed to get networks {err}");
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
                        network_id: network.id.unwrap(),
                        network_name: network.name,
                        assigned_ip: wireguard_network_device.wireguard_ip.to_string(),
                        endpoint: network.endpoint,
                        pubkey: network.pubkey,
                        allowed_ips,
                        dns: network.dns,
                    };
                    configs.push(config);
                }
            }

            let response = DeviceConfigResponse {
                device: Some(device.into()),
                configs,
                instance: Some(Instance::new(settings, self.config.url.clone()).into()),
            };

            Ok(Response::new(response))
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

impl From<User> for InitialUserInfo {
    fn from(user: User) -> Self {
        let is_active = user.has_password();
        Self {
            first_name: user.first_name,
            last_name: user.last_name,
            login: user.username,
            email: user.email,
            phone_number: user.phone,
            is_active,
        }
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
            created_at: device.created.timestamp(),
        }
    }
}

impl Enrollment {
    // Send configured welcome email to user after finishing enrollment
    async fn send_welcome_email(
        &self,
        transaction: &mut Transaction<'_, sqlx::Postgres>,
        mail_tx: &UnboundedSender<Mail>,
        user: &User,
        settings: &Settings,
        ip_address: String,
        device_info: Option<String>,
    ) -> Result<(), EnrollmentError> {
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
                Err(EnrollmentError::NotificationError(err.to_string()))
            }
        }
    }

    // Notify admin that a user has completed enrollment
    fn send_admin_notification(
        mail_tx: &UnboundedSender<Mail>,
        admin: &User,
        user: &User,
        ip_address: String,
        device_info: Option<String>,
    ) -> Result<(), EnrollmentError> {
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
                Err(EnrollmentError::NotificationError(err.to_string()))
            }
        }
    }
}
