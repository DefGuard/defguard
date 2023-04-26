use defguard::{
    build_webapp,
    db::{AppEvent, GatewayEvent, User},
    grpc::{worker::JobStatus, GatewayState, WorkerDetail, WorkerState},
    handlers::{
        worker::{JobData, Jobid},
        Auth,
    },
};
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::unbounded_channel;
mod common;
use common::init_test_db;

async fn make_client() -> (Client, Arc<Mutex<WorkerState>>) {
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

    let webapp = build_webapp(
        config,
        tx,
        rx,
        wg_tx,
        worker_state.clone(),
        gateway_state,
        pool,
    )
    .await;
    (Client::tracked(webapp).await.unwrap(), worker_state)
}

#[rocket::async_test]
async fn test_scheduling_worker_jobs() {
    let (client, worker_state) = make_client().await;

    // register a fake worker
    {
        let mut state = worker_state.lock().unwrap();
        state.register_worker("YubiBridge".to_string());
    };

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
    let user_job_id_1 = response.into_json::<Jobid>().await.unwrap().id;

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
    let admin_job_id_1 = response.into_json::<Jobid>().await.unwrap().id;

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
    let admin_job_id_2 = response.into_json::<Jobid>().await.unwrap().id;

    // add status for created jobs
    {
        let mut state = worker_state.lock().unwrap();
        state.set_job_status(
            JobStatus {
                id: "YubiBridge".to_string(),
                job_id: user_job_id_1,
                success: true,
                public_key: "".to_string(),
                ssh_key: "".to_string(),
                fingerprint: "".to_string(),
                error: "".to_string(),
            },
            "hpotter".into(),
        );
        state.set_job_status(
            JobStatus {
                id: "YubiBridge".to_string(),
                job_id: admin_job_id_1,
                success: true,
                public_key: "".to_string(),
                ssh_key: "".to_string(),
                fingerprint: "".to_string(),
                error: "".to_string(),
            },
            "hpotter".into(),
        );
        state.set_job_status(
            JobStatus {
                id: "YubiBridge".to_string(),
                job_id: admin_job_id_2,
                success: true,
                public_key: "".to_string(),
                ssh_key: "".to_string(),
                fingerprint: "".to_string(),
                error: "".to_string(),
            },
            "admin".into(),
        );
    };

    // admin can fetch status for all jobs
    let response = client
        .get(format!("/api/v1/worker/{}", user_job_id_1))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let response = client
        .get(format!("/api/v1/worker/{}", admin_job_id_1))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let response = client
        .get(format!("/api/v1/worker/{}", admin_job_id_2))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // // normal user can only fetch status of their own jobs
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client
        .get(format!("/api/v1/worker/{}", admin_job_id_1))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let response = client
        .get(format!("/api/v1/worker/{}", admin_job_id_2))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);

    let response = client
        .get(format!("/api/v1/worker/{}", user_job_id_1))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
}

#[rocket::async_test]
async fn test_worker_management_permissions() {
    let (client, worker_state) = make_client().await;

    // add some fake workers
    {
        let mut state = worker_state.lock().unwrap();
        state.register_worker("worker_1".into());
        state.register_worker("worker_2".into());
        state.register_worker("worker_3".into());
    }

    // admin can create worker tokens
    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/api/v1/worker/token").dispatch().await;
    assert_eq!(response.status(), Status::Created);

    // admin can list workers
    let response = client.get("/api/v1/worker").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let workers: Vec<WorkerDetail> = response.into_json().await.unwrap();
    assert_eq!(workers.len(), 3);

    // admin can remove a worker
    let response = client.delete("/api/v1/worker/worker_1").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let response = client.get("/api/v1/worker").dispatch().await;
    let workers: Vec<WorkerDetail> = response.into_json().await.unwrap();
    assert_eq!(workers.len(), 2);

    // normal user cannot create worker tokens
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/api/v1/worker/token").dispatch().await;
    assert_eq!(response.status(), Status::Forbidden);

    // normal user cannot list workers
    let response = client.get("/api/v1/worker").dispatch().await;
    assert_eq!(response.status(), Status::Forbidden);

    // normal user cannot remove a worker
    let response = client.delete("/api/v1/worker/worker_2").dispatch().await;
    assert_eq!(response.status(), Status::Forbidden);
}
