use crate::db::models::enrollment::Enrollment;
use crate::db::{DbPool, User};
use tonic::{Code, Request, Response, Status};
tonic::include_proto!("enrollment");

pub struct EnrollmentServer {
    pool: DbPool,
}

impl EnrollmentServer {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
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
        let Some(user) = User::find_by_id(&self.pool, enrollment.user_id)
            .await
            .map_err(|_| Status::new(Code::Internal, "unexpected error"))? else {
            error!("User not found for enrollment token {}", enrollment.id);
            return Err(Status::new(Code::Internal, "unexpected error"))
        };
        let Some(admin) = User::find_by_id(&self.pool, enrollment.admin_id)
            .await
            .map_err(|_| Status::new(Code::Internal, "unexpected error"))?else {
            error!("Admin not found for enrollment token {}", enrollment.id);
            return Err(Status::new(Code::Internal, "unexpected error"))
        };

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
        _request: Request<ActivateUserRequest>,
    ) -> Result<Response<()>, Status> {
        unimplemented!()
    }

    async fn create_device(
        &self,
        _request: Request<NewDevice>,
    ) -> Result<Response<CreateDeviceResponse>, Status> {
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
