use std::net::IpAddr;

use defguard_core::{
    db::Id,
    enterprise::{
        db::models::snat::UserSnatBinding,
        license::{get_cached_license, set_cached_license},
        snat::handlers::{EditUserSnatBinding, NewUserSnatBinding},
    },
    handlers::Auth,
};
use reqwest::StatusCode;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::common::{
    authenticate_admin, exceed_enterprise_limits, make_network, make_test_client, setup_pool,
};

#[sqlx::test]
async fn test_snat_crud(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;

    // admin login
    authenticate_admin(&client).await;

    // create location
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // list SNAT bindings (should be empty)
    let response = client.get("/api/v1/network/1/snat").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let bindings: Vec<UserSnatBinding<Id>> = response.json().await;
    assert!(bindings.is_empty());

    // create SNAT binding
    let new_binding = NewUserSnatBinding {
        user_id: 1, // admin user
        public_ip: "192.168.1.100".parse().unwrap(),
    };
    let response = client
        .post("/api/v1/network/1/snat")
        .json(&new_binding)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created_binding: UserSnatBinding<Id> = response.json().await;
    assert_eq!(created_binding.user_id, 1);
    assert_eq!(created_binding.location_id, 1);
    assert_eq!(
        created_binding.public_ip,
        "192.168.1.100".parse::<IpAddr>().unwrap()
    );

    // list SNAT bindings (should have one)
    let response = client.get("/api/v1/network/1/snat").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let bindings: Vec<UserSnatBinding<Id>> = response.json().await;
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].user_id, 1);
    assert_eq!(
        bindings[0].public_ip,
        "192.168.1.100".parse::<IpAddr>().unwrap()
    );

    // modify SNAT binding
    let edit_binding = EditUserSnatBinding {
        public_ip: "192.168.1.200".parse().unwrap(),
    };
    let response = client
        .put("/api/v1/network/1/snat/1")
        .json(&edit_binding)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated_binding: UserSnatBinding<Id> = response.json().await;
    assert_eq!(
        updated_binding.public_ip,
        "192.168.1.200".parse::<IpAddr>().unwrap()
    );

    // verify modification
    let response = client.get("/api/v1/network/1/snat").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let bindings: Vec<UserSnatBinding<Id>> = response.json().await;
    assert_eq!(bindings.len(), 1);
    assert_eq!(
        bindings[0].public_ip,
        "192.168.1.200".parse::<IpAddr>().unwrap()
    );

    // delete SNAT binding
    let response = client.delete("/api/v1/network/1/snat/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // verify deletion
    let response = client.get("/api/v1/network/1/snat").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let bindings: Vec<UserSnatBinding<Id>> = response.json().await;
    assert!(bindings.is_empty());
}

#[sqlx::test]
async fn test_snat_enterprise_required(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;

    // admin login
    authenticate_admin(&client).await;

    exceed_enterprise_limits(&client).await;

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // unset the license
    let license = get_cached_license().clone();
    set_cached_license(None);

    // try to use SNAT API without enterprise license
    let new_binding = NewUserSnatBinding {
        user_id: 1,
        public_ip: "192.168.1.100".parse().unwrap(),
    };

    let response = client.get("/api/v1/network/1/snat").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = client
        .post("/api/v1/network/1/snat")
        .json(&new_binding)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let edit_binding = EditUserSnatBinding {
        public_ip: "192.168.1.200".parse().unwrap(),
    };
    let response = client
        .put("/api/v1/network/1/snat/1")
        .json(&edit_binding)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = client.delete("/api/v1/network/1/snat/1").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // restore valid license and try again
    set_cached_license(license);

    let response = client.get("/api/v1/network/1/snat").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .post("/api/v1/network/1/snat")
        .json(&new_binding)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[sqlx::test]
async fn test_snat_admin_required(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;

    exceed_enterprise_limits(&client).await;

    // create network as admin
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // login as normal user
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // try to use SNAT API as normal user
    let new_binding = NewUserSnatBinding {
        user_id: 2, // hpotter user
        public_ip: "192.168.1.100".parse().unwrap(),
    };

    let response = client.get("/api/v1/network/1/snat").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = client
        .post("/api/v1/network/1/snat")
        .json(&new_binding)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let edit_binding = EditUserSnatBinding {
        public_ip: "192.168.1.200".parse().unwrap(),
    };
    let response = client
        .put("/api/v1/network/1/snat/2")
        .json(&edit_binding)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = client.delete("/api/v1/network/1/snat/2").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn test_snat_validation(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;

    // admin login
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    exceed_enterprise_limits(&client).await;

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // try to create binding for non-existent user
    let new_binding = NewUserSnatBinding {
        user_id: 999, // non-existent user
        public_ip: "192.168.1.100".parse().unwrap(),
    };
    let response = client
        .post("/api/v1/network/1/snat")
        .json(&new_binding)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // try to create binding for non-existent network
    let new_binding = NewUserSnatBinding {
        user_id: 1,
        public_ip: "192.168.1.100".parse().unwrap(),
    };
    let response = client
        .post("/api/v1/network/999/snat")
        .json(&new_binding)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // create valid binding
    let response = client
        .post("/api/v1/network/1/snat")
        .json(&new_binding)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // try to create duplicate binding (same user + location)
    let response = client
        .post("/api/v1/network/1/snat")
        .json(&new_binding)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);

    // try to modify non-existent binding
    let edit_binding = EditUserSnatBinding {
        public_ip: "192.168.1.200".parse().unwrap(),
    };
    let response = client
        .put("/api/v1/network/1/snat/999")
        .json(&edit_binding)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // try to delete non-existent binding
    let response = client.delete("/api/v1/network/1/snat/999").send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn test_snat_multiple_bindings(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;

    // admin login
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    exceed_enterprise_limits(&client).await;

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // create multiple SNAT bindings for different users
    let binding1 = NewUserSnatBinding {
        user_id: 1, // admin
        public_ip: "192.168.1.100".parse().unwrap(),
    };
    let response = client
        .post("/api/v1/network/1/snat")
        .json(&binding1)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let binding2 = NewUserSnatBinding {
        user_id: 2, // hpotter
        public_ip: "192.168.1.101".parse().unwrap(),
    };
    let response = client
        .post("/api/v1/network/1/snat")
        .json(&binding2)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // list all bindings
    let response = client.get("/api/v1/network/1/snat").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let bindings: Vec<UserSnatBinding<Id>> = response.json().await;
    assert_eq!(bindings.len(), 2);

    // verify both bindings exist
    let admin_binding = bindings.iter().find(|b| b.user_id == 1).unwrap();
    let user_binding = bindings.iter().find(|b| b.user_id == 2).unwrap();

    assert_eq!(
        admin_binding.public_ip,
        "192.168.1.100".parse::<IpAddr>().unwrap()
    );
    assert_eq!(
        user_binding.public_ip,
        "192.168.1.101".parse::<IpAddr>().unwrap()
    );

    // delete one binding
    let response = client
        .delete(format!("/api/v1/network/1/snat/{}", admin_binding.user_id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // verify only one binding remains
    let response = client.get("/api/v1/network/1/snat").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let bindings: Vec<UserSnatBinding<Id>> = response.json().await;
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].user_id, 2);
}
