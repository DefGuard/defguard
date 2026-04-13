use defguard_proto::{
    client_types::InstanceInfoRequest,
    proxy::{CoreRequest, core_request, core_response},
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::support::{
    assert_error_response, clear_test_license, complete_proxy_handshake, create_device_for_user,
    create_network, create_polling_token, create_user, create_user_with_device,
    set_test_license_business,
};
use crate::tests::common::HandlerTestContext;

#[sqlx::test]
async fn test_polling_returns_updated_device_config(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    // Polling requires a Business (or Enterprise) license.
    set_test_license_business();

    let _network = create_network(&context.pool).await;
    let (_user, device) = create_user_with_device(&context.pool).await;
    let token_str = create_polling_token(&context.pool, device.id).await;

    context.mock_proxy().send_request(CoreRequest {
        id: 10,
        device_info: None,
        payload: Some(core_request::Payload::InstanceInfo(InstanceInfoRequest {
            token: token_str.clone(),
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    match &response.payload {
        Some(core_response::Payload::InstanceInfo(info)) => {
            assert!(
                info.device_config.is_some(),
                "InstanceInfoResponse should contain a DeviceConfigResponse"
            );
        }
        other => panic!(
            "expected InstanceInfo response, got: {:?}",
            other.as_ref().map(std::mem::discriminant)
        ),
    }

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_polling_requires_business_license(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    // Explicitly clear any license — polling should be refused.
    clear_test_license();

    let (_user, device) = create_user_with_device(&context.pool).await;
    let token_str = create_polling_token(&context.pool, device.id).await;

    context.mock_proxy().send_request(CoreRequest {
        id: 11,
        device_info: None,
        payload: Some(core_request::Payload::InstanceInfo(InstanceInfoRequest {
            token: token_str,
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(
        code,
        tonic::Code::FailedPrecondition,
        "polling without a business license should return FailedPrecondition"
    );

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_polling_invalid_token_returns_error(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    set_test_license_business();

    context.mock_proxy().send_request(CoreRequest {
        id: 12,
        device_info: None,
        payload: Some(core_request::Payload::InstanceInfo(InstanceInfoRequest {
            token: "this-token-does-not-exist-00000000".to_string(),
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(
        code,
        tonic::Code::PermissionDenied,
        "invalid polling token should return PermissionDenied"
    );

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_polling_inactive_user_returns_error(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    set_test_license_business();

    // Create an inactive user.
    let mut user = create_user(&context.pool).await;
    user.is_active = false;
    user.save(&context.pool)
        .await
        .expect("failed to deactivate test user");

    let device = create_device_for_user(&context.pool, user.id).await;
    let token_str = create_polling_token(&context.pool, device.id).await;

    context.mock_proxy().send_request(CoreRequest {
        id: 13,
        device_info: None,
        payload: Some(core_request::Payload::InstanceInfo(InstanceInfoRequest {
            token: token_str,
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(
        code,
        tonic::Code::PermissionDenied,
        "polling for inactive user should return PermissionDenied"
    );

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_polling_reflects_network_changes(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    set_test_license_business();

    let _network = create_network(&context.pool).await;
    let (_user, device) = create_user_with_device(&context.pool).await;
    let token_str = create_polling_token(&context.pool, device.id).await;

    // First poll.
    context.mock_proxy().send_request(CoreRequest {
        id: 14,
        device_info: None,
        payload: Some(core_request::Payload::InstanceInfo(InstanceInfoRequest {
            token: token_str.clone(),
        })),
    });
    let first_response = context.mock_proxy_mut().recv_outbound().await;
    let first_info = match &first_response.payload {
        Some(core_response::Payload::InstanceInfo(info)) => info.clone(),
        _ => panic!("expected InstanceInfo on first poll"),
    };

    // Add a second network to expand the set of configs the device should see.
    let _network2 = create_network(&context.pool).await;
    // Re-run add_to_all_networks so the device gets a WireguardNetworkDevice
    // row for the newly created network (required for config-building).
    let mut conn = context.pool.acquire().await.expect("acquire connection");
    device
        .add_to_all_networks(&mut conn)
        .await
        .expect("add device to all networks");

    // Second poll with the same token.
    context.mock_proxy().send_request(CoreRequest {
        id: 15,
        device_info: None,
        payload: Some(core_request::Payload::InstanceInfo(InstanceInfoRequest {
            token: token_str.clone(),
        })),
    });
    let second_response = context.mock_proxy_mut().recv_outbound().await;
    let second_info = match &second_response.payload {
        Some(core_response::Payload::InstanceInfo(info)) => info.clone(),
        _ => panic!("expected InstanceInfo on second poll"),
    };

    // The second response should contain more network configs than the first.
    let first_cfg_count = first_info
        .device_config
        .as_ref()
        .map(|c| c.configs.len())
        .unwrap_or(0);
    let second_cfg_count = second_info
        .device_config
        .as_ref()
        .map(|c| c.configs.len())
        .unwrap_or(0);

    assert!(
        second_cfg_count > first_cfg_count,
        "second poll should reflect the new network; got first={first_cfg_count}, second={second_cfg_count}"
    );

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}
