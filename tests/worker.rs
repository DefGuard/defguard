use defguard::build_webapp;
use defguard::db::{AppEvent, GatewayEvent, User};
use defguard::grpc::{GatewayState, WorkerState};
use defguard::handlers::worker::JobData;
use defguard::handlers::Auth;
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::unbounded_channel;
mod common;
use common::init_test_db;

async fn make_client() -> Client {
    let (pool, config) = init_test_db().await;

    User::init_admin_user(&pool, &config.default_admin_password)
        .await
        .unwrap();

    let mut user = User::new(
        "hpotter".into(),
        "pass123",
        "Potter".into(),
        "Harry".into(),
        "h.potter@hogwart.edu.uk".into(),
        None,
    );
    user.save(&pool).await.unwrap();

    let (tx, rx) = unbounded_channel::<AppEvent>();
    let worker_state = Arc::new(Mutex::new(WorkerState::new(tx.clone())));
    let (wg_tx, wg_rx) = unbounded_channel::<GatewayEvent>();
    let gateway_state = Arc::new(Mutex::new(GatewayState::new(wg_rx)));

    let webapp = build_webapp(config, tx, rx, wg_tx, worker_state, gateway_state, pool).await;
    Client::tracked(webapp).await.unwrap()
}

#[rocket::async_test]
async fn test_scheduling_worker_jobs() {
    let client = make_client().await;

    // normal user can only provision keys for themselves
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let job_data = JobData {
        username: "hpotter".to_string(),
        worker: "YubiBridge".to_string(),
    };
    let response = client
        .post("/api/v1/worker/job")
        .json(&job_data)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);

    let job_data = JobData {
        username: "admin".to_string(),
        worker: "YubiBridge".to_string(),
    };
    let response = client
        .post("/api/v1/worker/job")
        .json(&job_data)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);

    // admin user can provision keys for other users
    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let job_data = JobData {
        username: "hpotter".to_string(),
        worker: "YubiBridge".to_string(),
    };
    let response = client
        .post("/api/v1/worker/job")
        .json(&job_data)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);

    let job_data = JobData {
        username: "admin".to_string(),
        worker: "YubiBridge".to_string(),
    };
    let response = client
        .post("/api/v1/worker/job")
        .json(&job_data)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);
}
