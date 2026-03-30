#[path = "handler/support.rs"]
mod support;

use defguard_common::db::models::{Device, User, polling_token::PollingToken};
use defguard_core::{
    db::models::enrollment::Token,
    grpc::GatewayEvent,
};
use defguard_proto::proxy::{
    AuthCallbackRequest, AuthFlowType, AuthInfoRequest, AwaitRemoteMfaFinishRequest,
    ClientMfaFinishRequest, ClientMfaOidcAuthenticateRequest, ClientMfaStartRequest,
    ClientMfaTokenValidationRequest, CoreRequest, ExistingDevice, InstanceInfoRequest, MfaMethod,
    NewDevice, core_request, core_response,
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use self::support::{
    assert_device_config_response, assert_error_response, assert_vpn_session_exists,
    clear_test_license, complete_proxy_handshake, create_device_for_user, create_enrollment_token,
    create_external_mfa_network, create_mfa_network, create_network, create_oidc_provider,
    create_polling_token, create_user, create_user_with_device, expect_bidi_mfa_success,
    expect_gateway_mfa_authorized, generate_totp_code, make_device_info, make_oidc_code,
    send_mfa_finish, send_mfa_start, send_token_validation, set_public_proxy_url,
    set_test_license_business, setup_user_email_mfa, setup_user_totp_mfa, start_enrollment_session,
};
use crate::tests::common::{HandlerTestContext, MockOidcProvider, TEST_TIMEOUT, reload_proxy};

include!("handler/lifecycle.rs");
include!("handler/enrollment.rs");
include!("handler/polling.rs");
include!("handler/mfa.rs");
include!("handler/oidc.rs");
