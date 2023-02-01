use crate::{
    appstate::AppState,
    auth::{AdminRole, Claims, ClaimsType, SessionInfo},
    db::User,
    error::OriWebError,
    grpc::WorkerState,
    handlers::{ApiResponse, ApiResult},
};

use rocket::{
    http::Status,
    serde::json::{serde_json::json, Json},
    State,
};
use std::sync::{Arc, Mutex};

#[derive(Deserialize)]
pub struct JobData {
    pub username: String,
    pub worker: String,
}

#[derive(Serialize)]
pub struct Jobid {
    pub id: u32,
}

#[derive(Serialize)]
struct JobResponseError {
    message: String,
}

#[post("/job", format = "json", data = "<data>")]
pub async fn create_job(
    _session: SessionInfo,
    appstate: &State<AppState>,
    data: Json<JobData>,
    worker_state: &State<Arc<Mutex<WorkerState>>>,
) -> ApiResult {
    let job_data = data.into_inner();
    match User::find_by_username(&appstate.pool, &job_data.username).await? {
        Some(user) => {
            let mut state = worker_state.lock().unwrap();
            debug!("Creating job");
            let id = state.create_job(
                &job_data.worker,
                user.first_name.clone(),
                user.last_name.clone(),
                user.email,
                job_data.username,
            );
            info!("Job created with id {}", id);
            Ok(ApiResponse {
                json: json!(Jobid { id }),
                status: Status::Created,
            })
        }
        None => Err(OriWebError::ObjectNotFound(format!(
            "user {} not found",
            job_data.username
        ))),
    }
}

#[get("/token", format = "json", rank = 2)]
pub async fn create_worker_token(session: SessionInfo, _admin: AdminRole) -> ApiResult {
    let username = session.user.username;
    let token = Claims::new(
        ClaimsType::YubiBridge,
        username,
        String::new(),
        u32::MAX.into(),
    )
    .to_jwt()
    .map_err(|_| OriWebError::Authorization("Failed to create bridge token".into()))?;
    Ok(ApiResponse {
        json: json!({ "token": token }),
        status: Status::Ok,
    })
}

#[get("/", format = "json")]
pub fn list_workers(
    _session: SessionInfo,
    worker_state: &State<Arc<Mutex<WorkerState>>>,
) -> ApiResult {
    let state = worker_state.lock().unwrap();
    let workers = state.list_workers();
    Ok(ApiResponse {
        json: json!(workers),
        status: Status::Ok,
    })
}

#[delete("/<worker_id>")]
pub async fn remove_worker(
    _session: SessionInfo,
    worker_state: &State<Arc<Mutex<WorkerState>>>,
    worker_id: &str,
) -> ApiResult {
    let mut state = worker_state.lock().unwrap();
    if state.remove_worker(worker_id) {
        Ok(ApiResponse::default())
    } else {
        error!("Worker {} not found", worker_id);
        Err(OriWebError::ObjectNotFound(format!(
            "worker_id {} not found",
            worker_id
        )))
    }
}

#[get("/<job_id>", format = "json")]
pub async fn job_status(
    _session: SessionInfo,
    worker_state: &State<Arc<Mutex<WorkerState>>>,
    job_id: u32,
) -> ApiResult {
    let state = worker_state.lock().unwrap();
    let job_response = state.get_job_status(job_id);
    if job_response.is_some() {
        if job_response.unwrap().success {
            Ok(ApiResponse {
                json: json!(job_response),
                status: Status::Ok,
            })
        } else {
            Ok(ApiResponse {
                json: json!(JobResponseError {
                    message: job_response.unwrap().error.clone()
                }),
                status: Status::NotFound,
            })
        }
    } else {
        Ok(ApiResponse {
            json: json!(job_response),
            status: Status::Ok,
        })
    }
}
