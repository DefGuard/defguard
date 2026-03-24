use claims::assert_matches;
use defguard_common::db::models::{AuthenticationKey, AuthenticationKeyType, User, YubiKey};
use defguard_core::db::AppEvent;
use defguard_proto::worker::{JobStatus, Worker, worker_service_client::WorkerServiceClient};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::sync::mpsc::error::TryRecvError;
use tonic::Code;

use super::common::{
    add_authorization_metadata, create_gateway_jwt, make_grpc_test_server, setup_grpc_pool,
    worker_request,
};

#[sqlx::test]
async fn register_worker_success_and_duplicate(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_grpc_pool(PgPoolOptions::new(), options).await;
    let server = make_grpc_test_server(&pool).await;
    let mut client = WorkerServiceClient::new(server.client_channel.clone());

    let response = client
        .register_worker(worker_request(
            Worker {
                id: "worker-1".into(),
            },
            "admin",
        ))
        .await;

    assert!(response.is_ok());

    let status = client
        .register_worker(worker_request(
            Worker {
                id: "worker-1".into(),
            },
            "admin",
        ))
        .await
        .expect_err("duplicate worker should fail");

    assert_eq!(status.code(), Code::AlreadyExists);
    assert_eq!(status.message(), "Worker already registered");
}

#[sqlx::test]
async fn get_job_returns_not_found_for_unknown_worker(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_grpc_pool(PgPoolOptions::new(), options).await;
    let server = make_grpc_test_server(&pool).await;
    let mut client = WorkerServiceClient::new(server.client_channel.clone());

    let status = client
        .get_job(worker_request(
            Worker {
                id: "missing-worker".into(),
            },
            "admin",
        ))
        .await
        .expect_err("missing worker should not have jobs");

    assert_eq!(status.code(), Code::NotFound);
    assert_eq!(status.message(), "No more jobs");
}

#[sqlx::test]
async fn get_job_returns_not_found_for_registered_worker_without_jobs(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_grpc_pool(PgPoolOptions::new(), options).await;
    let server = make_grpc_test_server(&pool).await;
    let mut client = WorkerServiceClient::new(server.client_channel.clone());

    client
        .register_worker(worker_request(
            Worker {
                id: "worker-1".into(),
            },
            "admin",
        ))
        .await
        .expect("worker registration should succeed");

    let status = client
        .get_job(worker_request(
            Worker {
                id: "worker-1".into(),
            },
            "admin",
        ))
        .await
        .expect_err("worker without jobs should get not found");

    assert_eq!(status.code(), Code::NotFound);
    assert_eq!(status.message(), "No more jobs");
}

#[sqlx::test]
async fn get_job_returns_seeded_payload(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_grpc_pool(PgPoolOptions::new(), options).await;
    let server = make_grpc_test_server(&pool).await;
    let mut client = WorkerServiceClient::new(server.client_channel.clone());

    client
        .register_worker(worker_request(
            Worker {
                id: "worker-1".into(),
            },
            "admin",
        ))
        .await
        .expect("worker registration should succeed");

    let job_id = {
        let mut state = server.worker_state.lock().unwrap();
        state.create_job(
            "worker-1",
            "Minerva".into(),
            "McGonagall".into(),
            "minerva@hogwart.edu.uk".into(),
            "hpotter".into(),
        )
    };

    let response = client
        .get_job(worker_request(
            Worker {
                id: "worker-1".into(),
            },
            "admin",
        ))
        .await
        .expect("seeded job should be returned")
        .into_inner();

    assert_eq!(response.job_id, job_id);
    assert_eq!(response.first_name, "Minerva");
    assert_eq!(response.last_name, "McGonagall");
    assert_eq!(response.email, "minerva@hogwart.edu.uk");
}

#[sqlx::test]
async fn set_job_done_success_removes_job_and_stores_status(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_grpc_pool(PgPoolOptions::new(), options).await;
    let mut server = make_grpc_test_server(&pool).await;
    let mut client = WorkerServiceClient::new(server.client_channel.clone());

    client
        .register_worker(worker_request(
            Worker {
                id: "worker-1".into(),
            },
            "admin",
        ))
        .await
        .expect("worker registration should succeed");

    let job_id = {
        let mut state = server.worker_state.lock().unwrap();
        state.create_job(
            "worker-1",
            "Harry".into(),
            "Potter".into(),
            "h.potter@hogwart.edu.uk".into(),
            "hpotter".into(),
        )
    };

    client
        .set_job_done(worker_request(
            JobStatus {
                id: "worker-1".into(),
                job_id,
                success: true,
                public_key: "gpg-public-key".into(),
                ssh_key: "ssh-public-key".into(),
                yubikey_serial: "yk-serial-1".into(),
                error: String::new(),
            },
            "admin",
        ))
        .await
        .expect("job completion should succeed");

    {
        let mut state = server.worker_state.lock().unwrap();
        let status = state
            .get_job_status(job_id)
            .expect("job status should be recorded");
        assert!(status.success);
        assert_eq!(status.serial, "yk-serial-1");
        assert_eq!(status.error, "");
        assert!(
            state
                .get_job("worker-1", std::net::IpAddr::from([127, 0, 0, 1]))
                .is_none()
        );
    }

    let user = User::find_by_username(&pool, "hpotter")
        .await
        .expect("user query should succeed")
        .expect("user should exist");
    let yubikeys = YubiKey::find_by_user_id(&pool, user.id)
        .await
        .expect("yubikey query should succeed");
    let auth_keys = AuthenticationKey::find_by_user_id(&pool, user.id, None)
        .await
        .expect("auth key query should succeed");

    assert_eq!(yubikeys.len(), 1);
    assert_eq!(yubikeys[0].serial, "yk-serial-1");
    assert_eq!(auth_keys.len(), 2);
    assert!(auth_keys.iter().any(|key| {
        key.key_type == AuthenticationKeyType::Ssh
            && key.key == "ssh-public-key"
            && key.yubikey_id == Some(yubikeys[0].id)
    }));
    assert!(auth_keys.iter().any(|key| {
        key.key_type == AuthenticationKeyType::Gpg
            && key.key == "gpg-public-key"
            && key.yubikey_id == Some(yubikeys[0].id)
    }));

    let event = server
        .app_event_rx
        .try_recv()
        .expect("success should emit an app event");
    assert_matches!(
        event,
        AppEvent::HWKeyProvision(data)
            if data.username == "hpotter"
                && data.email == "h.potter@hogwart.edu.uk"
                && data.ssh_key == "ssh-public-key"
                && data.pgp_key == "gpg-public-key"
                && data.serial == "yk-serial-1"
    );
    assert_matches!(server.app_event_rx.try_recv(), Err(TryRecvError::Empty));
}

#[sqlx::test]
async fn set_job_done_failure_stores_status_without_keys_or_event(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_grpc_pool(PgPoolOptions::new(), options).await;
    let mut server = make_grpc_test_server(&pool).await;
    let mut client = WorkerServiceClient::new(server.client_channel.clone());

    client
        .register_worker(worker_request(
            Worker {
                id: "worker-1".into(),
            },
            "admin",
        ))
        .await
        .expect("worker registration should succeed");

    let job_id = {
        let mut state = server.worker_state.lock().unwrap();
        state.create_job(
            "worker-1",
            "Harry".into(),
            "Potter".into(),
            "h.potter@hogwart.edu.uk".into(),
            "hpotter".into(),
        )
    };

    client
        .set_job_done(worker_request(
            JobStatus {
                id: "worker-1".into(),
                job_id,
                success: false,
                public_key: "gpg-public-key".into(),
                ssh_key: "ssh-public-key".into(),
                yubikey_serial: "yk-serial-1".into(),
                error: "worker failed".into(),
            },
            "admin",
        ))
        .await
        .expect("failed completion should still return ok");

    {
        let state = server.worker_state.lock().unwrap();
        let status = state
            .get_job_status(job_id)
            .expect("failure status should be recorded");
        assert!(!status.success);
        assert_eq!(status.error, "worker failed");
    }

    let user = User::find_by_username(&pool, "hpotter")
        .await
        .expect("user query should succeed")
        .expect("user should exist");
    assert!(
        YubiKey::find_by_user_id(&pool, user.id)
            .await
            .expect("yubikey query should succeed")
            .is_empty()
    );
    assert!(
        AuthenticationKey::find_by_user_id(&pool, user.id, None)
            .await
            .expect("auth key query should succeed")
            .is_empty()
    );
    assert_matches!(server.app_event_rx.try_recv(), Err(TryRecvError::Empty));
}

#[sqlx::test]
async fn set_job_done_unknown_job_is_ignored(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_grpc_pool(PgPoolOptions::new(), options).await;
    let mut server = make_grpc_test_server(&pool).await;
    let mut client = WorkerServiceClient::new(server.client_channel.clone());

    client
        .register_worker(worker_request(
            Worker {
                id: "worker-1".into(),
            },
            "admin",
        ))
        .await
        .expect("worker registration should succeed");

    client
        .set_job_done(worker_request(
            JobStatus {
                id: "worker-1".into(),
                job_id: 999,
                success: true,
                public_key: "gpg-public-key".into(),
                ssh_key: "ssh-public-key".into(),
                yubikey_serial: "yk-serial-1".into(),
                error: String::new(),
            },
            "admin",
        ))
        .await
        .expect("unknown jobs should be ignored");

    {
        let state = server.worker_state.lock().unwrap();
        assert!(state.get_job_status(999).is_none());
    }

    let user = User::find_by_username(&pool, "hpotter")
        .await
        .expect("user query should succeed")
        .expect("user should exist");
    assert!(
        YubiKey::find_by_user_id(&pool, user.id)
            .await
            .expect("yubikey query should succeed")
            .is_empty()
    );
    assert!(
        AuthenticationKey::find_by_user_id(&pool, user.id, None)
            .await
            .expect("auth key query should succeed")
            .is_empty()
    );
    assert_matches!(server.app_event_rx.try_recv(), Err(TryRecvError::Empty));
}

#[sqlx::test]
async fn worker_interceptor_requires_valid_yubibridge_token(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_grpc_pool(PgPoolOptions::new(), options).await;
    let server = make_grpc_test_server(&pool).await;
    let mut client = WorkerServiceClient::new(server.client_channel.clone());

    let missing_auth = client
        .register_worker(tonic::Request::new(Worker {
            id: "worker-missing-auth".into(),
        }))
        .await
        .expect_err("missing auth should fail");
    assert_eq!(missing_auth.code(), Code::Unauthenticated);
    assert_eq!(missing_auth.message(), "Missing authorization header");

    let mut invalid_request = tonic::Request::new(Worker {
        id: "worker-invalid-token".into(),
    });
    add_authorization_metadata(&mut invalid_request, "not-a-jwt");
    let invalid_token = client
        .register_worker(invalid_request)
        .await
        .expect_err("invalid token should fail");
    assert_eq!(invalid_token.code(), Code::Unauthenticated);
    assert_eq!(invalid_token.message(), "Invalid token");

    let mut wrong_claims_request = tonic::Request::new(Worker {
        id: "worker-wrong-claims".into(),
    });
    add_authorization_metadata(
        &mut wrong_claims_request,
        &create_gateway_jwt("admin", "gateway-network-1"),
    );
    let wrong_claims = client
        .register_worker(wrong_claims_request)
        .await
        .expect_err("wrong claims type should fail");
    assert_eq!(wrong_claims.code(), Code::Unauthenticated);
    assert_eq!(wrong_claims.message(), "Invalid token");

    let allowed = client
        .get_job(worker_request(
            Worker {
                id: "worker-valid-token".into(),
            },
            "admin",
        ))
        .await
        .expect_err("valid token should reach service logic");
    assert_eq!(allowed.code(), Code::NotFound);
    assert_eq!(allowed.message(), "No more jobs");
}
