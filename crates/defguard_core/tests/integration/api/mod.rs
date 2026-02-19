mod acl;
mod api_tokens;
mod auth;
mod common;
mod enrollment;
mod enterprise_settings;
mod forward_auth;
mod group;
mod location_stats;
mod oauth;
mod openid;
mod openid_login;
mod proxy;
mod settings;
mod snat;
mod user;
mod webhook;
mod wireguard;
mod wireguard_network_allowed_groups;
mod wireguard_network_devices;
mod wireguard_network_import;
// FIXME(mwojcik): rewrite for new stats implementation
// mod wireguard_network_stats;
mod worker;

const TEST_SERVER_URL: &str = "http://localhost:3000/";
