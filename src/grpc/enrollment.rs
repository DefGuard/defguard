use crate::db::models::enrollment::Enrollment;
use crate::db::{DbPool, User};
use tonic::{Request, Response, Status};
tonic::include_proto!("enrollment");

pub struct EnrollmentServer {
    pool: DbPool,
}

impl EnrollmentServer {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // check if token provided with request corresponds to a valid enrollment session
    async fn validate_session<T>(&self, request: Request<T>) -> Result<Enrollment, Status> {
        debug!("Validating enrollment session token");
        let token = match request.metadata().get("authorization") {
            Some(token) => token
                .to_str()
                .map_err(|_| Status::unauthenticated("Invalid token"))?,
            None => return Err(Status::unauthenticated("Missing authorization header")),
        };

        let enrollment = Enrollment::find_by_id(&self.pool, token).await?;

        if enrollment.is_session_valid() {
            Ok(enrollment)
        } else {
            Err(Status::unauthenticated("Enrollment session expired"))
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
        let enrollment = self.validate_session(request).await?;

        // fetch related users
        let user = enrollment.fetch_user(&self.pool).await?;
        info!("Activating user account for {}", user.username);
        unimplemented!()
    }

    async fn create_device(
        &self,
        request: Request<NewDevice>,
    ) -> Result<Response<CreateDeviceResponse>, Status> {
        debug!("Adding new user device");
        let enrollment = self.validate_session(request).await?;

        // fetch related users
        let user = enrollment.fetch_user(&self.pool).await?;
        info!("dding new device for user {}", user.username);
        unimplemented!()
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
