use crate::{
    config::DefGuardConfig,
    db::{
        models::{
            device::{DeviceConfig, DeviceInfo},
            enrollment::{Enrollment, EnrollmentError},
        },
        DbPool, Device, GatewayEvent, Settings, User,
    },
    handlers::user::check_password_strength,
    ldap::utils::ldap_add_user,
    mail::Mail,
    templates,
};
use sqlx::Transaction;
use tokio::sync::{broadcast::Sender, mpsc::UnboundedSender};
use tonic::{Request, Response, Status};

pub mod proto {
    tonic::include_proto!("enrollment");
}
use proto::{
    enrollment_service_server, ActivateUserRequest, AdminInfo, CreateDeviceResponse,
    Device as ProtoDevice, DeviceConfig as ProtoDeviceConfig, EnrollmentStartRequest,
    EnrollmentStartResponse, InitialUserInfo, NewDevice,
};

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
        // FIXME: check if LDAP feature is enabled
        let ldap_feature_active = true;
        Self {
            pool,
            wireguard_tx,
            mail_tx,
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
            let _result = ldap_add_user(&self.config, &user, &request.password).await;
        };

        let settings = Settings::get_settings(&mut *transaction)
            .await
            .map_err(|_| {
                error!("Failed to get settings");
                Status::internal("unexpected error")
            })?;

        // send welcome email
        enrollment
            .send_welcome_email(&mut transaction, &self.mail_tx, &user, &settings)
            .await?;

        // send success notification to admin
        let admin = enrollment.fetch_admin(&mut *transaction).await?;
        Enrollment::send_admin_notification(&self.mail_tx, &admin, &user)?;

        transaction.commit().await.map_err(|_| {
            error!("Failed to commit transaction");
            Status::internal("unexpected error")
        })?;

        Ok(Response::new(()))
    }

    async fn create_device(
        &self,
        request: Request<NewDevice>,
    ) -> Result<Response<CreateDeviceResponse>, Status> {
        debug!("Adding new user device: {request:?}");
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

        transaction.commit().await.map_err(|_| {
            error!("Failed to commit transaction");
            Status::internal("unexpected error")
        })?;

        let response = CreateDeviceResponse {
            device: Some(device.into()),
            configs: configs.into_iter().map(Into::into).collect(),
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
            phone_number: user.phone,
        }
    }
}

impl From<DeviceConfig> for ProtoDeviceConfig {
    fn from(config: DeviceConfig) -> Self {
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

impl Enrollment {
    // Send configured welcome email to user after finishing enrollment
    async fn send_welcome_email(
        &self,
        transaction: &mut Transaction<'_, sqlx::Postgres>,
        mail_tx: &UnboundedSender<Mail>,
        user: &User,
        settings: &Settings,
    ) -> Result<(), EnrollmentError> {
        debug!("Sending welcome mail to {}", user.username);
        let mail = Mail {
            to: user.email.clone(),
            subject: settings.enrollment_welcome_email_subject.clone().unwrap(),
            content: self.get_welcome_email_content(&mut *transaction).await?,
            attachments: Vec::new(),
            result_tx: None,
        };
        match mail_tx.send(mail) {
            Ok(_) => {
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
    ) -> Result<(), EnrollmentError> {
        debug!(
            "Sending enrollment success notification for user {} to {}",
            user.username, admin.username
        );
        let mail = Mail {
            to: admin.email.clone(),
            subject: "[defguard] User enrollment completed".into(),
            content: templates::enrollment_admin_notification(user, admin)?,
            attachments: Vec::new(),
            result_tx: None,
        };
        match mail_tx.send(mail) {
            Ok(_) => {
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
