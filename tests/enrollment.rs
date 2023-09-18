mod common;

use axum::http::StatusCode;
use defguard::{
    db::{models::enrollment::Enrollment, DbPool},
    handlers::{AddUserData, Auth},
};
use serde::Deserialize;
use serde_json::json;

use self::common::{client::TestClient, make_test_client};

async fn make_client() -> (TestClient, DbPool) {
    let (client, client_state) = make_test_client().await;
    (client, client_state.pool)
}

#[tokio::test]
async fn test_initialize_enrollment() {
    let (client, pool) = make_client().await;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create user with password
    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: Some("1234".into()),
        password: Some("Password1234543$!".into()),
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // verify enrollment token was not created
    let enrollments = Enrollment::fetch_all(&pool).await.unwrap();
    assert_eq!(enrollments.len(), 0);

    // try to start enrollment
    let response = client
        .post("/api/v1/user/adumbledore/start_enrollment")
        .json(&json!({}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

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
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // verify enrollment token was not created
    let enrollments = Enrollment::fetch_all(&pool).await.unwrap();
    assert_eq!(enrollments.len(), 0);

    // try to start enrollment
    let response = client
        .post("/api/v1/user/adumbledore2/start_enrollment")
        .json(&json!({}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response: StartEnrollmentResponse = response.json().await;

    // verify enrollment token was created
    let enrollment = Enrollment::find_by_id(&pool, &response.enrollment_token)
        .await
        .unwrap();
    assert_eq!(enrollment.user_id, 4);
    assert_eq!(enrollment.admin_id, 1);
    assert_eq!(enrollment.used_at, None);
}
