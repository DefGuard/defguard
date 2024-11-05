use std::sync::{Arc, Mutex};

use axum::{
    extract::{Extension, Json, Path, State},
    http::StatusCode,
};
use serde_json::json;

use super::ApiResponse;
use crate::{
    appstate::AppState,
    auth::{AdminRole, Claims, ClaimsType, SessionInfo},
    db::models::user::User,
    error::WebError,
    grpc::WorkerState,
};

#[derive(Deserialize, Serialize)]
pub struct JobData {
    pub username: String,
    pub worker: String,
}

#[derive(Deserialize, Serialize)]
pub struct Jobid {
    pub id: u32,
}

#[derive(Serialize)]
struct JobResponseError {
    message: String,
}

pub(crate) async fn create_job(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Extension(worker_state): Extension<Arc<Mutex<WorkerState>>>,
    Json(job_data): Json<JobData>,
) -> Result<ApiResponse, WebError> {
    let (worker, username) = (job_data.worker.clone(), job_data.username.clone());
    debug!(
        "User {} creating a worker job for worker {worker} and user {username}",
        session.user.username,
    );
    if let Some(user) = User::find_by_username(&appstate.pool, &job_data.username).await? {
        // only admins should be able to create jobs for other users
        if user != session.user && !session.is_admin {
            warn!(
                "User {} cannot schedule jobs for other users",
                session.user.username
            );
            return Err(WebError::Forbidden(
                "Cannot schedule jobs for other users.".into(),
            ));
        };

        let mut state = worker_state.lock().unwrap();
        debug!("Creating job");
        let id = state.create_job(
            &job_data.worker,
            user.first_name.clone(),
            user.last_name.clone(),
            user.email,
            job_data.username,
        );
        info!(
            "User {} created a worker job (ID {id}) for worker {worker} and user {username}",
            session.user.username,
        );
        Ok(ApiResponse {
            json: json!(Jobid { id }),
            status: StatusCode::CREATED,
        })
    } else {
        error!("Failed to create job, user {} not found", job_data.username);
        Err(WebError::ObjectNotFound(format!(
            "user {} not found",
            job_data.username
        )))
    }
}

pub(crate) async fn create_worker_token(
    session: SessionInfo,
    _admin: AdminRole,
) -> Result<ApiResponse, WebError> {
    let username = session.user.username;
    let token = Claims::new(
        ClaimsType::YubiBridge,
        username,
        String::new(),
        u32::MAX.into(),
    )
    .to_jwt()
    .map_err(|_| WebError::Authorization("Failed to create bridge token".into()))?;
    Ok(ApiResponse {
        json: json!({ "token": token }),
        status: StatusCode::CREATED,
    })
}

pub(crate) async fn list_workers(
    _admin: AdminRole,
    Extension(worker_state): Extension<Arc<Mutex<WorkerState>>>,
) -> Result<ApiResponse, WebError> {
    debug!("Listing workers");
    let state = worker_state.lock().unwrap();
    let workers = state.list_workers();
    debug!("Listed workers");
    Ok(ApiResponse {
        json: json!(workers),
        status: StatusCode::OK,
    })
}

pub(crate) async fn remove_worker(
    _admin: AdminRole,
    session: SessionInfo,
    Extension(worker_state): Extension<Arc<Mutex<WorkerState>>>,
    Path(id): Path<String>,
) -> Result<ApiResponse, WebError> {
    debug!("User {} deleting worker {id}", session.user.username,);
    let mut state = worker_state.lock().unwrap();
    if state.remove_worker(&id) {
        info!("User {} deleted worker {id}", session.user.username);
        Ok(ApiResponse::default())
    } else {
        error!("Worker {id} not found");
        Err(WebError::ObjectNotFound(format!(
            "worker_id {id} not found",
        )))
    }
}

pub(crate) async fn job_status(
    session: SessionInfo,
    Extension(worker_state): Extension<Arc<Mutex<WorkerState>>>,
    Path(id): Path<u32>,
) -> Result<ApiResponse, WebError> {
    debug!(
        "User {} fetching job status for job {id}",
        session.user.username
    );
    let state = worker_state.lock().unwrap();
    let job_response = state.get_job_status(id);
    if let Some(response) = job_response {
        // prevent non-admin users from accessing other users' jobs status
        if !session.is_admin && response.username != session.user.username {
            warn!(
                "User {} cannot fetch job status for other users' jobs",
                session.user.username
            );
            return Err(WebError::Forbidden(
                "Cannot fetch job status for other users' jobs.".into(),
            ));
        }
        if response.success {
            debug!("Fetched job status for job {id}");
            Ok(ApiResponse {
                json: json!(job_response),
                status: StatusCode::OK,
            })
        } else {
            error!(
                "Failed to fetch job status for job {id}: {}",
                response.error
            );
            Ok(ApiResponse {
                json: json!(JobResponseError {
                    message: response.error.clone()
                }),
                status: StatusCode::NOT_FOUND,
            })
        }
    } else {
        debug!("Fetched job status for job {id}");
        Ok(ApiResponse {
            json: json!(job_response),
            status: StatusCode::OK,
        })
    }
}
