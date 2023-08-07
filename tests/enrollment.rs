use defguard::{
    db::{models::enrollment::Enrollment, DbPool},
    handlers::{AddUserData, Auth},
};
use rocket::{http::Status, local::asynchronous::Client, serde::Deserialize};
use serde_json::json;

mod common;
use crate::common::make_test_client;

async fn make_client() -> (Client, DbPool) {
    let (client, client_state) = make_test_client().await;
    (client, client_state.pool)
}

#[rocket::async_test]
async fn test_initialize_enrollment() {
    let (client, pool) = make_client().await;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // create user with password
    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: Some("1234".into()),
        password: Some("Password1234543$!".into()),
        send_enrollment_notification: false,
    };
    let response = client.post("/api/v1/user").json(&new_user).dispatch().await;
    assert_eq!(response.status(), Status::Created);
    assert_eq!(response.into_string().await.unwrap(), "{}");

    // verify enrollment token was not created
    let enrollments = Enrollment::fetch_all(&pool).await.unwrap();
    assert_eq!(enrollments.len(), 0);

    // try to start enrollment
    let response = client
        .post("/api/v1/user/adumbledore/start_enrollment")
        .json(&json!({}))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);

    // create user without password
    #[derive(Deserialize)]
    struct StartEnrollmentResponse {
        enrollment_token: String,
    }
    let new_user = AddUserData {
        username: "adumbledore2".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: Some("1234".into()),
        password: None,
        send_enrollment_notification: false,
    };
    let response = client.post("/api/v1/user").json(&new_user).dispatch().await;
    assert_eq!(response.status(), Status::Created);

    // verify enrollment token was not created
    let enrollments = Enrollment::fetch_all(&pool).await.unwrap();
    assert_eq!(enrollments.len(), 0);

    // try to start enrollment
    let response = client
        .post("/api/v1/user/adumbledore2/start_enrollment")
        .json(&json!({}))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);
    let response: StartEnrollmentResponse = response.into_json().await.unwrap();

    // verify enrollment token was created
    let enrollment = Enrollment::find_by_id(&pool, &response.enrollment_token)
        .await
        .unwrap();
    assert_eq!(enrollment.user_id, 4);
    assert_eq!(enrollment.admin_id, 1);
    assert_eq!(enrollment.used_at, None);
}
