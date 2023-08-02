use crate::config::DefGuardConfig;
use crate::handlers::user::check_password_strength;
use crate::ldap::utils::ldap_add_user;
use crate::license::{Features, License};
use crate::{
    db::{
        models::{device::DeviceInfo, enrollment::Enrollment},
        DbPool, Device, GatewayEvent, User,
    },
    handlers, templates,
};
use tokio::sync::broadcast::Sender;
use tokio::sync::mpsc::UnboundedSender;
use tonic::{Request, Response, Status};

pub mod proto {
    tonic::include_proto!("enrollment");
}
use crate::db::Settings;
use crate::mail::Mail;
use proto::{
    enrollment_service_server, ActivateUserRequest, AdminInfo, CreateDeviceResponse,
    Device as ProtoDevice, DeviceConfig, EnrollmentStartRequest, EnrollmentStartResponse,
    InitialUserInfo, NewDevice,
};

const ENROLLMENT_WELCOME_MAIL_SUBJECT: &str = "Welcome to Defguard";

pub struct EnrollmentServer {
    pool: DbPool,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
    config: DefGuardConfig,
    ldap_feature_active: bool,
}

impl EnrollmentServer {
    #[must_use]
    pub fn new(
        pool: DbPool,
        wireguard_tx: Sender<GatewayEvent>,
        mail_tx: UnboundedSender<Mail>,
        config: DefGuardConfig,
    ) -> Self {
        // check if LDAP feature is enabled
        let license_decoded = License::decode(&config.license);
        let ldap_feature_active = license_decoded.validate(&Features::Ldap);
        Self {
            pool,
            wireguard_tx,
            mail_tx,
            config,
            ldap_feature_active,
        }
    }

    // check if token provided with request corresponds to a valid enrollment session
    async fn validate_session<T>(&self, request: &Request<T>) -> Result<Enrollment, Status> {
        debug!("Validating enrollment session token");
        let token = match request.metadata().get("authorization") {
            Some(token) => token
                .to_str()
                .map_err(|_| Status::unauthenticated("Invalid token"))?,
            None => {
                error!("Missing authorization header in request");
                return Err(Status::unauthenticated("Missing authorization header"));
            }
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
            error!("Error sending wireguard event {}", err);
        }
    }
}

#[tonic::async_trait]
impl enrollment_service_server::EnrollmentService for EnrollmentServer {
    async fn start_enrollment(
        &self,
        request: Request<EnrollmentStartRequest>,
    ) -> Result<Response<EnrollmentStartResponse>, Status> {
        debug!("Starting enrollment session");
        let request = request.into_inner();
        // fetch enrollment token
        let mut enrollment = Enrollment::find_by_id(&self.pool, &request.token).await?;

        // fetch related users
        let user = enrollment.fetch_user(&self.pool).await?;
        let admin = enrollment.fetch_admin(&self.pool).await?;

        // validate token & start session
        info!("Starting enrollment session for user {}", user.username);
        let session_deadline = enrollment
            .start_session(&self.pool, self.config.enrollment_session_timeout.as_secs())
            .await?;

        let response = EnrollmentStartResponse {
            admin: Some(admin.into()),
            user: Some(user.into()),
            deadline_timestamp: session_deadline.timestamp(),
            final_page_content: "<h1>Hi there!</h1>".to_string(),
            vpn_setup_optional: false,
        };

        Ok(Response::new(response))
    }

    async fn activate_user(
        &self,
        request: Request<ActivateUserRequest>,
    ) -> Result<Response<()>, Status> {
        debug!("Activating user account");
        let enrollment = self.validate_session(&request).await?;

        // check if password is strong enough
        let request = request.into_inner();
        if let Err(err) = check_password_strength(&request.password) {
            error!("Password not strong enough: {}", err);
            return Err(Status::invalid_argument("password not strong enough"));
        }

        // fetch related users
        let mut user = enrollment.fetch_user(&self.pool).await?;
        info!("Activating user account for {}", user.username);
        if user.has_password() {
            error!("User {} already activated", user.username);
            return Err(Status::invalid_argument("user already activated"));
        }

        // update user
        user.phone = Some(request.phone_number);
        user.set_password(&request.password);
        user.save(&self.pool).await.map_err(|err| {
            error!("Failed to update user {}: {}", user.username, err);
            Status::internal("unexpected error")
        })?;

        // sync with LDAP
        if self.ldap_feature_active {
            let _result = ldap_add_user(&self.config, &user, &request.password).await;
        };

        // send welcome email
        debug!("Sending welcome mail to {}", user.username);
        let settings = Settings::find_by_id(&self.pool, 1)
            .await
            .map_err(|err| {
                error!("Failed to get settings: {err}");
                Status::internal("unexpected error")
            })?
            .ok_or_else(|| {
                error!("Failed to get settings");
                Status::internal("unexpected error")
            })?;
        let content = match settings.enrollment_use_welcome_message_as_email {
            true => settings.enrollment_welcome_message,
            false => settings.enrollment_welcome_email,
        }
        .ok_or_else(|| {
            error!("Welcome message not configured");
            Status::internal("unexpected error")
        })?;

        let mail = Mail {
            to: user.email.clone(),
            subject: ENROLLMENT_WELCOME_MAIL_SUBJECT.to_string(),
            content: templates::enrollment_welcome_mail(&content).map_err(|err| {
                error!(
                    "Failed to render welcome email for user {}: {}",
                    user.username, err
                );
                Status::internal("unexpected error")
            })?,
        };
        match self.mail_tx.send(mail.clone()) {
            Ok(_) => {
                info!("Sent enrollment welcome mail to {}", user.username);
            }
            Err(err) => {
                error!("Error sending welcome mail: {mail:?}: {err}");
                Status::internal("unexpected error");
            }
        }

        Ok(Response::new(()))
    }

    async fn create_device(
        &self,
        request: Request<NewDevice>,
    ) -> Result<Response<CreateDeviceResponse>, Status> {
        debug!("Adding new user device");
        let enrollment = self.validate_session(&request).await?;

        // fetch related users
        let user = enrollment.fetch_user(&self.pool).await?;

        // add device
        info!("Adding new device for user {}", user.username);
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
        device.save(&mut transaction).await.map_err(|err| {
            error!("Failed to save device {}: {}", device.name, err);
            Status::internal("unexpected error")
        })?;

        let (network_info, configs) = device
            .add_to_all_networks(&mut transaction, &self.config.admin_groupname)
            .await
            .map_err(|err| {
                error!(
                    "Failed to add device {} to existing networks: {}",
                    device.name, err
                );
                Status::internal("unexpected error")
            })?;

        self.send_wireguard_event(GatewayEvent::DeviceCreated(DeviceInfo {
            device: device.clone(),
            network_info,
        }));

        transaction.commit().await.map_err(|_| {
            error!("Failed to commit transaction");
            Status::internal("unexpected error")
        })?;

        let response = CreateDeviceResponse {
            device: Some(device.into()),
            configs: configs.into_iter().map(|config| config.into()).collect(),
        };

        Ok(Response::new(response))
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
        Self {
            first_name: user.first_name,
            last_name: user.last_name,
            login: user.username,
            email: user.email,
        }
    }
}

impl From<handlers::wireguard::DeviceConfig> for DeviceConfig {
    fn from(config: handlers::wireguard::DeviceConfig) -> Self {
        Self {
            network_id: config.network_id,
            network_name: config.network_name,
            config: config.config,
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
