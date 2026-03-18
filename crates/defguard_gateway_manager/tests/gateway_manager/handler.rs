#[path = "handler/support.rs"]
mod support;

use defguard_common::db::models::device::{DeviceInfo, WireguardNetworkDevice};
use defguard_core::grpc::GatewayEvent;
use defguard_proto::gateway::{UpdateType, core_response};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tonic::Status;

use self::support::{
    assert_device_event_for_different_network_is_ignored,
    assert_device_event_is_ignored_before_config_handshake, assert_firewall_disable_update,
    assert_firewall_event_for_different_network_is_ignored, assert_firewall_modify_update,
    assert_network_create_update, assert_network_delete_update, assert_network_modify_update,
    assert_peer_update, assert_send_ok, build_test_firewall_config,
    create_authorized_mfa_device_for_current_network, create_authorized_mfa_device_for_network,
    create_device_for_network, create_device_info_for_current_network,
    enable_internal_mfa_for_network, expected_keepalive_interval, panic_unexpected, parse_test_ip,
};
use crate::common::{HandlerTestContext, build_peer_stats, reload_gateway};

include!("handler/handshake.rs");
include!("handler/lifecycle.rs");
include!("handler/stats.rs");
include!("handler/network_events.rs");
include!("handler/firewall_events.rs");
include!("handler/device_events.rs");
include!("handler/mfa.rs");
