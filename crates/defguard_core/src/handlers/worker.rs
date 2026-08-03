use std::sync::{Arc, Mutex};

use axum::{
    extract::{Extension, Json, Path, State},
    http::StatusCode,
};
use defguard_common::{
    auth::claims::{Claims, ClaimsType},
    db::models::User,
};
use serde_json::json;
use utoipa::ToSchema;

use super::{ApiErrorResponse, ApiResponse, ApiResult};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    error::WebError,
    grpc::WorkerState,
};

#[derive(Deserialize, Serialize, ToSchema)]
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

/// Create a YubiKey provisioning job.
#[utoipa::path(
    post,
    path = "/api/v1/worker/job",
    tag = "worker",
    request_body = JobData,
    responses(
        (status = 201, description = "Job created, returns the job ID.", body = Object, example = json!({"id": 1})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges or the request must target your own account.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to create job.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn create_job(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Extension(worker_state): Extension<Arc<Mutex<WorkerState>>>,
    Json(job_data): Json<JobData>,
) -> ApiResult {
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
            return Err(WebError::Forbidden("Cannot schedule jobs for other users"));
        }

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
        Ok(ApiResponse::json(Jobid { id }, StatusCode::CREATED))
    } else {
        error!("Failed to create job, user {} not found", job_data.username);
        Err(WebError::ObjectNotFound(format!(
            "user {} not found",
            job_data.username
        )))
    }
}

/// Create a token used by a provisioning worker to register itself.
#[utoipa::path(
    get,
    path = "/api/v1/worker/token",
    tag = "worker",
    responses(
        (status = 200, description = "Worker token.", body = Object),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to create worker token.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn create_worker_token(session: SessionInfo, _admin: AdminRole) -> ApiResult {
    let username = session.user.username;
    let token = Claims::new(
        ClaimsType::YubiBridge,
        username,
        String::new(),
        u32::MAX.into(),
    )
    .to_jwt()
    .map_err(|_| WebError::Authorization("Failed to create bridge token".into()))?;
    Ok(ApiResponse::new(
        json!({ "token": token }),
        StatusCode::CREATED,
    ))
}

/// List registered provisioning workers.
#[utoipa::path(
    get,
    path = "/api/v1/worker/",
    tag = "worker",
    responses(
        (status = 200, description = "List of registered workers.", body = Object),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to list workers.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn list_workers(
    _admin: AdminRole,
    Extension(worker_state): Extension<Arc<Mutex<WorkerState>>>,
) -> ApiResult {
    debug!("Listing workers");
    let state = worker_state.lock().unwrap();
    let workers = state.list_workers();
    debug!("Listed workers");
    Ok(ApiResponse::json(workers, StatusCode::OK))
}

/// Remove a provisioning worker.
#[utoipa::path(
    delete,
    path = "/api/v1/worker/{id}",
    tag = "worker",
    params(
        ("id" = String, Path, description = "ID of worker"),
    ),
    responses(
        (status = 200, description = "Worker removed.", body = Object, example = json!({})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Worker not found.", body = ApiErrorResponse, example = json!({"msg": "worker not found"})),
        (status = 500, description = "Unable to remove worker.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn remove_worker(
    _admin: AdminRole,
    session: SessionInfo,
    Extension(worker_state): Extension<Arc<Mutex<WorkerState>>>,
    Path(id): Path<String>,
) -> ApiResult {
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

/// Get the status of a YubiKey provisioning job.
#[utoipa::path(
    get,
    path = "/api/v1/worker/{id}",
    tag = "worker",
    params(
        ("id" = i32, Path, description = "ID of job"),
    ),
    responses(
        (status = 200, description = "Job status.", body = Object),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 404, description = "Job not found.", body = ApiErrorResponse, example = json!({"msg": "job not found"})),
        (status = 500, description = "Unable to get job status.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn job_status(
    session: SessionInfo,
    Extension(worker_state): Extension<Arc<Mutex<WorkerState>>>,
    Path(id): Path<u32>,
) -> ApiResult {
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
                "Cannot fetch job status for other users' jobs",
            ));
        }
        if response.success {
            debug!("Fetched job status for job {id}");
            Ok(ApiResponse::json(job_response, StatusCode::OK))
        } else {
            error!(
                "Failed to fetch job status for job {id}: {}",
                response.error
            );
            Ok(ApiResponse::json(
                JobResponseError {
                    message: response.error.clone(),
                },
                StatusCode::NOT_FOUND,
            ))
        }
    } else {
        debug!("Fetched job status for job {id}");
        Ok(ApiResponse::json(job_response, StatusCode::OK))
    }
}
