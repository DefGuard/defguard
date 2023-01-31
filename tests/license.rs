use defguard::grpc::{GatewayState, WorkerState};
#[cfg(feature = "worker")]
use defguard::handlers::worker::{create_job, job_status, list_workers, remove_worker};
use defguard::{
    build_webapp,
    db::{AppEvent, GatewayEvent},
    handlers::Auth,
    license::{Features, License},
};
use rocket::{http::Status, local::asynchronous::Client, routes};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::unbounded_channel;

mod common;
use common::{init_test_db, LICENSE_ENTERPRISE, LICENSE_EXPIRED, LICENSE_WITHOUT_OPENID};

async fn make_client(license: &str) -> Client {
    let (pool, mut config) = init_test_db().await;
    config.license = license.into();

    let (tx, rx) = unbounded_channel::<AppEvent>();
    let (wg_tx, wg_rx) = unbounded_channel::<GatewayEvent>();
    let (webhook_tx, _webhook_rx) = unbounded_channel::<AppEvent>();
    let gateway_state = Arc::new(Mutex::new(GatewayState::new(wg_rx)));

    let webapp = build_webapp(config, tx, rx, wg_tx, gateway_state, pool).await;

    let worker_state = Arc::new(Mutex::new(WorkerState::new(webhook_tx.clone())));
    let license_decoded = License::decode(license);
    #[cfg(feature = "worker")]
    let webapp = if license_decoded.validate(&Features::Worker) {
        webapp.manage(worker_state).mount(
            "/api/v1/worker",
            routes![create_job, list_workers, job_status, remove_worker],
        )
    } else {
        webapp
    };
    let client = Client::tracked(webapp).await.unwrap();
    {
        let auth = Auth::new("admin".into(), "pass123".into());
        let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
    }
    client
}

#[cfg(feature = "openid")]
#[rocket::async_test]
async fn test_license_ok() {
    let client = make_client(LICENSE_ENTERPRISE).await;

    // Check if openid path exist
    let response = client.get("/api/v1/oauth").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // check if worker path exist
    let response = client.get("/api/v1/worker").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client
        .get("/.well-known/openid-configuration")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
}

#[ignore]
#[rocket::async_test]
async fn test_license_expired() {
    // test expired license
    let client = make_client(LICENSE_EXPIRED).await;

    let response = client.get("/api/v1/oauth").dispatch().await;
    assert_eq!(response.status(), Status::NotFound);

    let response = client.get("/api/v1/worker").dispatch().await;
    assert_eq!(response.status(), Status::NotFound);

    let response = client
        .get("/.well-known/openid-configuration")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::NotFound);
}

#[ignore]
#[cfg(feature = "openid")]
#[rocket::async_test]
async fn test_license_openid_disabled() {
    // test license without OpenID
    let client = make_client(LICENSE_WITHOUT_OPENID).await;

    let response = client.get("/api/v1/oauth").dispatch().await;
    assert_eq!(response.status(), Status::NotFound);

    let response = client.get("/api/v1/worker").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client
        .get("/.well-known/openid-configuration")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::NotFound);
}
