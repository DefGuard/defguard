use crate::handlers::user::check_password_strength;
use crate::{
    db::{
        models::{device::DeviceInfo, enrollment::Enrollment},
        DbPool, Device, GatewayEvent, User,
    },
    handlers,
};
use tokio::sync::broadcast::Sender;
use tonic::{Request, Response, Status};
tonic::include_proto!("enrollment");

pub struct EnrollmentServer {
    pool: DbPool,
    wireguard_tx: Sender<GatewayEvent>,
    admin_groupname: String,
}

impl EnrollmentServer {
    #[must_use]
    pub fn new(pool: DbPool, wireguard_tx: Sender<GatewayEvent>, admin_groupname: String) -> Self {
        Self {
            pool,
            wireguard_tx,
            admin_groupname,
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

        if enrollment.is_session_valid() {
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
        let session_deadline = enrollment.start_session(&self.pool).await?;

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
        todo!();

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
            .add_to_all_networks(&mut transaction, &self.admin_groupname)
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
