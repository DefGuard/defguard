use crate::db::DbPool;
use tonic::Status;

use super::proto::{
    ClientMfaFinishRequest, ClientMfaFinishResponse, ClientMfaStartRequest, ClientMfaStartResponse,
};

pub(super) struct ClientMfaServer {
    pool: DbPool,
}

impl ClientMfaServer {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn start_client_mfa_login(
        &self,
        request: ClientMfaStartRequest,
    ) -> Result<ClientMfaStartResponse, Status> {
        todo!()
    }

    pub async fn finish_client_mfa_login(
        &self,
        request: ClientMfaFinishRequest,
    ) -> Result<ClientMfaFinishResponse, Status> {
        todo!()
    }
}
