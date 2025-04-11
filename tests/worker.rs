pub mod common;

use common::{make_client_with_state, setup_pool};
use defguard::{
    grpc::{worker::JobStatus, WorkerDetail},
    handlers::{
        worker::{JobData, Jobid},
        Auth,
    },
};
use reqwest::StatusCode;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

#[sqlx::test]
async fn test_scheduling_worker_jobs(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, state) = make_client_with_state(pool).await;

    // register a fake worker
    {
        let mut state = state.worker_state.lock().unwrap();
        state.register_worker("YubiBridge".to_string());
    };

    // normal user can only provision keys for themselves
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let job_data = JobData {
        username: "hpotter".to_string(),
        worker: "YubiBridge".to_string(),
    };
    let response = client
        .post("/api/v1/worker/job")
        .json(&job_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let user_job_id_1 = response.json::<Jobid>().await.id;

    let job_data = JobData {
        username: "admin".to_string(),
        worker: "YubiBridge".to_string(),
    };
    let response = client
        .post("/api/v1/worker/job")
        .json(&job_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // admin user can provision keys for other users
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let job_data = JobData {
        username: "hpotter".to_string(),
        worker: "YubiBridge".to_string(),
    };
    let response = client
        .post("/api/v1/worker/job")
        .json(&job_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let admin_job_id_1 = response.json::<Jobid>().await.id;

    let job_data = JobData {
        username: "admin".to_string(),
        worker: "YubiBridge".to_string(),
    };
    let response = client
        .post("/api/v1/worker/job")
        .json(&job_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let admin_job_id_2 = response.json::<Jobid>().await.id;

    // add status for created jobs
    {
        let mut state = state.worker_state.lock().unwrap();
        state.set_job_status(
            JobStatus {
                id: "YubiBridge".to_string(),
                job_id: user_job_id_1,
                success: true,
                public_key: String::new(),
                ssh_key: String::new(),
                yubikey_serial: String::new(),
                error: String::new(),
            },
            "hpotter".into(),
        );
        state.set_job_status(
            JobStatus {
                id: "YubiBridge".to_string(),
                job_id: admin_job_id_1,
                success: true,
                public_key: String::new(),
                ssh_key: String::new(),
                yubikey_serial: String::new(),
                error: String::new(),
            },
            "hpotter".into(),
        );
        state.set_job_status(
            JobStatus {
                id: "YubiBridge".to_string(),
                job_id: admin_job_id_2,
                success: true,
                public_key: String::new(),
                ssh_key: String::new(),
                yubikey_serial: String::new(),
                error: String::new(),
            },
            "admin".into(),
        );
    };

    // admin can fetch status for all jobs
    let response = client
        .get(format!("/api/v1/worker/{user_job_id_1}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .get(format!("/api/v1/worker/{admin_job_id_1}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .get(format!("/api/v1/worker/{admin_job_id_2}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // // normal user can only fetch status of their own jobs
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .get(format!("/api/v1/worker/{admin_job_id_1}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .get(format!("/api/v1/worker/{admin_job_id_2}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = client
        .get(format!("/api/v1/worker/{user_job_id_1}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_worker_management_permissions(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, state) = make_client_with_state(pool).await;

    // add some fake workers
    {
        let mut state = state.worker_state.lock().unwrap();
        state.register_worker("worker_1".into());
        state.register_worker("worker_2".into());
        state.register_worker("worker_3".into());
    }

    // admin can create worker tokens
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/worker/token").send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // admin can list workers
    let response = client.get("/api/v1/worker").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let workers: Vec<WorkerDetail> = response.json().await;
    assert_eq!(workers.len(), 3);

    // admin can remove a worker
    let response = client.delete("/api/v1/worker/worker_1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client.get("/api/v1/worker").send().await;
    let workers: Vec<WorkerDetail> = response.json().await;
    assert_eq!(workers.len(), 2);

    // normal user cannot create worker tokens
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/worker/token").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // normal user cannot list workers
    let response = client.get("/api/v1/worker").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // normal user cannot remove a worker
    let response = client.delete("/api/v1/worker/worker_2").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
