use defguard::handlers::Auth;
use rocket::{http::Status, local::asynchronous::Client};

mod common;
use common::{
    make_license_test_client, LICENSE_ENTERPRISE, LICENSE_EXPIRED, LICENSE_WITHOUT_OPENID,
};

async fn make_client(license: &str) -> Client {
    let (client, _) = make_license_test_client(license).await;

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
