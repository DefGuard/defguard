pub mod common;

use std::sync::{Arc, Mutex};

use defguard::{
    grpc::{worker::JobStatus, WorkerDetail, WorkerState},
    handlers::{
        worker::{JobData, Jobid},
        Auth,
    },
};
use reqwest::StatusCode;

use self::common::{client::TestClient, make_test_client};

async fn make_client() -> (TestClient, Arc<Mutex<WorkerState>>) {
    let (client, client_status) = make_test_client().await;
    (client, client_status.worker_state)
}

#[tokio::test]
async fn test_scheduling_worker_jobs() {
    let (client, worker_state) = make_client().await;

    // register a fake worker
    {
        let mut state = worker_state.lock().unwrap();
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
        let mut state = worker_state.lock().unwrap();
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

#[tokio::test]
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
