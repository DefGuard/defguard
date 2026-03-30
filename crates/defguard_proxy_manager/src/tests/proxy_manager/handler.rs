#[path = "handler/support.rs"]
mod support;

use defguard_common::db::models::{Device, polling_token::PollingToken};
use defguard_core::grpc::GatewayEvent;
use defguard_proto::proxy::{
    CoreRequest, ExistingDevice, InstanceInfoRequest, NewDevice,
    core_request, core_response,
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use self::support::{
    assert_device_config_response, assert_error_response, clear_test_license, complete_proxy_handshake,
    create_device_for_user, create_enrollment_token, create_network, create_polling_token,
    create_user, create_user_with_device, make_device_info, set_test_license_business,
    start_enrollment_session,
};
use crate::tests::common::{HandlerTestContext, TEST_TIMEOUT, reload_proxy};

include!("handler/lifecycle.rs");
include!("handler/enrollment.rs");
include!("handler/polling.rs");
