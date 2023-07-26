use crate::db::DbPool;
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
}

#[tonic::async_trait]
impl enrollment_service_server::EnrollmentService for EnrollmentServer {
    async fn start_enrollment(
        &self,
        request: Request<EnrollmentStartRequest>,
    ) -> Result<Response<EnrollmentStartResponse>, Status> {
        unimplemented!()
    }

    async fn activate_user(
        &self,
        request: Request<ActivateUserRequest>,
    ) -> Result<Response<()>, Status> {
        unimplemented!()
    }

    async fn create_device(
        &self,
        request: Request<NewDevice>,
    ) -> Result<Response<CreateDeviceResponse>, Status> {
        unimplemented!()
    }
}
