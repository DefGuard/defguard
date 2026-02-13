#![allow(clippy::too_many_arguments)]
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, LazyLock, Mutex, RwLock},
};

use anyhow::anyhow;
use axum::{
    Extension, Json, Router,
    http::{Request, StatusCode},
    routing::{delete, get, post, put},
    serve,
};
use defguard_certs::CertificateAuthority;
use defguard_common::{
    VERSION,
    auth::claims::{Claims, ClaimsType},
    config::{DefGuardConfig, InitVpnLocationArgs, server_config},
    db::{
        init_db,
        models::{
            Device, DeviceType, Settings, User, WireguardNetwork,
            oauth2client::OAuth2Client,
            settings::{initialize_current_settings, update_current_settings},
            wireguard::{
                DEFAULT_DISCONNECT_THRESHOLD, DEFAULT_KEEPALIVE_INTERVAL, DEFAULT_WIREGUARD_MTU,
                LocationMfaMode, ServiceLocationMode,
            },
        },
    },
    types::proxy::ProxyControlMessage,
};
use defguard_version::server::DefguardVersionLayer;
use defguard_web_ui::{index, svg, web_asset};
use events::ApiEvent;
use handlers::{
    activity_log::get_activity_log_events,
    auth::disable_user_mfa,
    component_setup::setup_proxy_tls_stream,
    group::{bulk_assign_to_groups, list_groups_info},
    network_devices::{
        add_network_device, check_ip_availability, download_network_device_config,
        find_available_ips, get_network_device, list_network_devices, modify_network_device,
        start_network_device_setup, start_network_device_setup_for_device,
    },
    ssh_authorized_keys::{
        add_authentication_key, delete_authentication_key, fetch_authentication_keys,
        rename_authentication_key,
    },
    updates::check_new_version,
    wireguard::{all_gateways_status, networks_overview_stats},
    yubikey::{delete_yubikey, rename_yubikey},
};
use ipnetwork::IpNetwork;
use regex::Regex;
use secrecy::ExposeSecret;
use semver::Version;
use sqlx::PgPool;
use tokio::{
    net::TcpListener,
    sync::{
        broadcast::Sender,
        mpsc::{UnboundedReceiver, UnboundedSender},
    },
};
use tower_http::{
    set_header::SetResponseHeaderLayer,
    trace::{DefaultOnResponse, TraceLayer},
};
use tracing::Level;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    appstate::AppState,
    auth::failed_login::FailedLoginMap,
    db::AppEvent,
    enterprise::{
        handlers::{
            acl::{
                alias::{
                    create_acl_alias, delete_acl_alias, get_acl_alias, list_acl_aliases,
                    update_acl_alias,
                },
                apply_acl_aliases, apply_acl_rules, create_acl_rule, delete_acl_rule,
                destination::{
                    create_acl_destination, delete_acl_destination, get_acl_destination,
                    list_acl_destinations, update_acl_destination,
                },
                get_acl_rule, list_acl_rules, update_acl_rule,
            },
            activity_log_stream::{
                create_activity_log_stream, delete_activity_log_stream, get_activity_log_stream,
                modify_activity_log_stream,
            },
            api_tokens::{add_api_token, delete_api_token, fetch_api_tokens, rename_api_token},
            check_enterprise_info,
            enterprise_settings::{get_enterprise_settings, patch_enterprise_settings},
            openid_login::{auth_callback, get_auth_info},
            openid_providers::{
                add_openid_provider, delete_openid_provider, get_openid_provider,
                list_openid_providers, modify_openid_provider, test_dirsync_connection,
            },
        },
        snat::handlers::{
            create_snat_binding, delete_snat_binding, list_snat_bindings, modify_snat_binding,
        },
    },
    grpc::{GatewayEvent, WorkerState},
    handlers::{
        app_info::get_app_info,
        auth::{
            authenticate, email_mfa_code, email_mfa_disable, email_mfa_enable, email_mfa_init,
            logout, mfa_disable, mfa_enable, recovery_code, request_email_mfa_code, totp_code,
            totp_disable, totp_enable, totp_secret, webauthn_end, webauthn_finish, webauthn_init,
            webauthn_start,
        },
        component_setup::setup_gateway_tls_stream,
        forward_auth::forward_auth,
        group::{
            add_group_member, create_group, delete_group, get_group, list_groups, modify_group,
            remove_group_member,
        },
        mail::{send_support_data, test_mail},
        openid_clients::{
            add_openid_client, change_openid_client, change_openid_client_state,
            delete_openid_client, get_openid_client, list_openid_clients,
        },
        openid_flow::{
            authorization, discovery_keys, openid_configuration, secure_authorization, token,
            userinfo,
        },
        proxy::{delete_proxy, proxy_details, proxy_list, update_proxy},
        settings::{
            get_settings, get_settings_essentials, patch_settings, set_default_branding,
            test_ldap_settings, update_settings,
        },
        ssh_authorized_keys::get_authorized_keys,
        support::{configuration, logs},
        updates::outdated_components,
        user::{
            add_user, change_password, change_self_password, delete_authorized_app,
            delete_security_key, delete_user, get_user, list_users, me, modify_user,
            reset_password, start_enrollment, start_remote_desktop_configuration,
            username_available,
        },
        webhooks::{
            add_webhook, change_enabled, change_webhook, delete_webhook, get_webhook, list_webhooks,
        },
        wireguard::{
            add_device, add_user_devices, change_gateway, create_network, delete_device,
            delete_network, devices_stats, download_config, gateway_status, get_device,
            import_network, list_devices, list_networks, list_user_devices, modify_device,
            modify_network, network_details, network_stats, remove_gateway,
        },
        worker::{create_job, create_worker_token, job_status, list_workers, remove_worker},
    },
    location_management::sync_location_allowed_devices,
    version::IncompatibleComponents,
};

pub mod appstate;
pub mod auth;
pub mod db;
pub mod enrollment_management;
pub mod enterprise;
pub mod error;
pub mod events;
pub mod grpc;
pub mod handlers;
pub mod headers;
pub mod location_management;
pub mod support;
pub mod updates;
pub mod user_management;
pub mod utility_thread;
pub mod version;
pub mod wg_config;

#[macro_use]
extern crate tracing;

#[macro_use]
extern crate serde;

static PHONE_NUMBER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\+?\d{1,3}\s?)?(\(\d{1,3}\)|\d{1,3})[-\s]?\d{1,4}[-\s]?\d{1,4}?$")
        .expect("Failed to parse phone number regex")
});

mod openapi;

/// Simple health-check.
pub async fn health_check() -> &'static str {
    "alive"
}

pub async fn handle_404() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not found")
}

async fn openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(openapi::ApiDoc::openapi())
}

pub fn build_webapp(
    webhook_tx: UnboundedSender<AppEvent>,
    webhook_rx: UnboundedReceiver<AppEvent>,
    wireguard_tx: Sender<GatewayEvent>,
    worker_state: Arc<Mutex<WorkerState>>,
    pool: PgPool,
    failed_logins: Arc<Mutex<FailedLoginMap>>,
    event_tx: UnboundedSender<ApiEvent>,
    version: Version,
    incompatible_components: Arc<RwLock<IncompatibleComponents>>,
    proxy_control_tx: tokio::sync::mpsc::Sender<ProxyControlMessage>,
) -> Router {
    let webapp: Router<AppState> = Router::new()
        .route("/", get(index))
        .route("/{*path}", get(index))
        .route("/fonts/{*path}", get(web_asset))
        .route("/assets/{*path}", get(web_asset))
        .route("/svg/{*path}", get(svg))
        .fallback_service(get(handle_404));

    let webapp = webapp.nest(
        "/api/v1",
        Router::new()
            .route("/health", get(health_check))
            .route("/info", get(get_app_info))
            .route("/ssh_authorized_keys", get(get_authorized_keys))
            .route("/api-docs", get(openapi))
            .route("/updates", get(check_new_version))
            // /auth
            .route("/auth", post(authenticate))
            .route("/auth/logout", post(logout))
            .route("/auth/mfa", put(mfa_enable).delete(mfa_disable))
            .route("/auth/webauthn/init", post(webauthn_init))
            .route("/auth/webauthn/finish", post(webauthn_finish))
            .route("/auth/webauthn/start", post(webauthn_start))
            .route("/auth/webauthn", post(webauthn_end))
            .route("/auth/totp/init", post(totp_secret))
            .route("/auth/totp", post(totp_enable))
            .route("/auth/totp/verify", post(totp_code))
            .route("/auth/email/init", post(email_mfa_init))
            .route(
                "/auth/email",
                get(request_email_mfa_code).post(email_mfa_enable),
            )
            .route("/auth/email/verify", post(email_mfa_code))
            .route("/auth/recovery", post(recovery_code))
            // /user
            .route("/user", get(list_users).post(add_user))
            .route("/user/{username}", get(get_user))
            .route("/user/{username}/start_enrollment", post(start_enrollment))
            .route(
                "/user/{username}/start_desktop",
                post(start_remote_desktop_configuration),
            )
            .route("/user/available", post(username_available))
            .route("/user/{username}", put(modify_user).delete(delete_user))
            // FIXME: username `change_password` is invalid
            .route("/user/change_password", put(change_self_password))
            .route("/user/{username}/password", put(change_password))
            .route("/user/{username}/reset_password", post(reset_password))
            // disable mfa
            .route("/user/{username}/email", delete(email_mfa_disable))
            .route("/user/{username}/totp", delete(totp_disable))
            // auth keys
            .route(
                "/user/{username}/auth_key",
                get(fetch_authentication_keys).post(add_authentication_key),
            )
            .route(
                "/user/{username}/auth_key/{key_id}",
                delete(delete_authentication_key),
            )
            .route(
                "/user/{username}/auth_key/{key_id}/rename",
                post(rename_authentication_key),
            )
            // yubi keys
            .route("/user/{username}/yubikey/{key_id}", delete(delete_yubikey))
            .route(
                "/user/{username}/yubikey/{key_id}/rename",
                post(rename_yubikey),
            )
            // API tokens
            .route(
                "/user/{username}/api_token",
                get(fetch_api_tokens).post(add_api_token),
            )
            .route(
                "/user/{username}/api_token/{token_id}",
                delete(delete_api_token),
            )
            .route(
                "/user/{username}/api_token/{token_id}/rename",
                post(rename_api_token),
            )
            .route(
                "/user/{username}/security_key/{id}",
                delete(delete_security_key),
            )
            .route("/me", get(me))
            .route(
                "/user/{username}/oauth_app/{oauth2client_id}",
                delete(delete_authorized_app),
            )
            .route("/user/{username}/mfa", delete(disable_user_mfa))
            // forward_auth
            .route("/forward_auth", get(forward_auth))
            // group
            .route("/group", get(list_groups).post(create_group))
            .route(
                "/group/{name}",
                get(get_group)
                    .put(modify_group)
                    .delete(delete_group)
                    .post(add_group_member),
            )
            .route("/group/{name}/user/{username}", delete(remove_group_member))
            .route("/group-info", get(list_groups_info))
            .route("/groups-assign", post(bulk_assign_to_groups))
            // mail
            .route("/mail/test", post(test_mail))
            .route("/mail/support", post(send_support_data))
            // settings
            .route(
                "/settings",
                get(get_settings).put(update_settings).patch(patch_settings),
            )
            .route("/settings/{id}", put(set_default_branding))
            // settings for frontend
            .route("/settings_essentials", get(get_settings_essentials))
            // enterprise settings
            .route(
                "/settings_enterprise",
                get(get_enterprise_settings).patch(patch_enterprise_settings),
            )
            // support
            .route("/support/configuration", get(configuration))
            .route("/support/logs", get(logs))
            // webhooks
            .route("/webhook", post(add_webhook).get(list_webhooks))
            .route(
                "/webhook/{id}",
                get(get_webhook)
                    .put(change_webhook)
                    .delete(delete_webhook)
                    .post(change_enabled),
            )
            // ldap
            .route("/ldap/test", get(test_ldap_settings))
            // activity log
            .route("/activity_log", get(get_activity_log_events))
            // Proxy routes
            .route("/proxy", get(proxy_list))
            .route(
                "/proxy/{proxy_id}",
                get(proxy_details).put(update_proxy).delete(delete_proxy),
            )
            // Proxy setup with SSE
            .route("/proxy/setup/stream", get(setup_proxy_tls_stream)),
    );

    // Enterprise features
    let webapp = webapp.nest(
        "/api/v1/openid",
        Router::new()
            .route(
                "/provider",
                get(list_openid_providers).post(add_openid_provider),
            )
            .route(
                "/provider/{name}",
                get(get_openid_provider)
                    .put(modify_openid_provider)
                    .delete(delete_openid_provider),
            )
            .route("/callback", post(auth_callback))
            .route("/auth_info", get(get_auth_info)),
    );

    let webapp = webapp.nest(
        "/api/v1",
        Router::new()
            .route("/enterprise_info", get(check_enterprise_info))
            .route("/test_directory_sync", get(test_dirsync_connection)),
    );

    // activity log stream
    let webapp = webapp.nest(
        "/api/v1/activity_log_stream",
        Router::new()
            .route(
                "/",
                get(get_activity_log_stream).post(create_activity_log_stream),
            )
            .route(
                "/{id}",
                delete(delete_activity_log_stream).put(modify_activity_log_stream),
            ),
    );

    let webapp = webapp
        .nest(
            "/api/v1/oauth",
            Router::new()
                .route("/discovery/keys", get(discovery_keys))
                .route("/", post(add_openid_client).get(list_openid_clients))
                .route(
                    "/{client_id}",
                    get(get_openid_client)
                        .put(change_openid_client)
                        .post(change_openid_client_state)
                        .delete(delete_openid_client),
                )
                .route("/authorize", get(authorization).post(secure_authorization))
                .route("/token", post(token))
                .route("/userinfo", get(userinfo)),
        )
        .route(
            "/.well-known/openid-configuration",
            get(openid_configuration),
        );

    let webapp = webapp.nest(
        "/api/v1/acl",
        Router::new()
            .route("/rule", get(list_acl_rules).post(create_acl_rule))
            .route("/rule/apply", put(apply_acl_rules))
            .route(
                "/rule/{id}",
                get(get_acl_rule)
                    .put(update_acl_rule)
                    .delete(delete_acl_rule),
            )
            .route("/alias", get(list_acl_aliases).post(create_acl_alias))
            .route(
                "/alias/{id}",
                get(get_acl_alias)
                    .put(update_acl_alias)
                    .delete(delete_acl_alias),
            )
            .route("/alias/apply", put(apply_acl_aliases))
            .route(
                "/destination",
                get(list_acl_destinations).post(create_acl_destination),
            )
            .route(
                "/destination/{id}",
                get(get_acl_destination)
                    .put(update_acl_destination)
                    .delete(delete_acl_destination),
            ),
    );

    let webapp = webapp.nest(
        "/api/v1",
        Router::new()
            // FIXME: Conflict; change /device/{device_id} to /device/{username}.
            .route("/device/{device_id}", post(add_device))
            .route(
                "/device/{device_id}",
                put(modify_device).get(get_device).delete(delete_device),
            )
            .route("/device", get(list_devices))
            .route("/device/user/{username}", get(list_user_devices))
            // Network devices, as opposed to user devices
            .route(
                "/device/network",
                post(add_network_device).get(list_network_devices),
            )
            .route(
                "/device/network/ip/{network_id}",
                get(find_available_ips).post(check_ip_availability),
            )
            .route(
                "/device/network/{device_id}",
                put(modify_network_device)
                    .get(get_network_device)
                    .delete(delete_device),
            )
            .route(
                "/device/network/{device_id}/config",
                get(download_network_device_config),
            )
            .route(
                "/device/network/start_cli",
                post(start_network_device_setup),
            )
            .route(
                "/device/network/start_cli/{device_id}",
                post(start_network_device_setup_for_device),
            )
            .route("/network", post(create_network).get(list_networks))
            .route("/network/import", post(import_network))
            .route("/network/stats", get(networks_overview_stats))
            .route("/network/gateways", get(all_gateways_status))
            .route(
                "/network/{network_id}",
                put(modify_network)
                    .delete(delete_network)
                    .get(network_details),
            )
            // Gateway adding (uses SSE)
            .route(
                "/network/{network_id}/gateways/setup",
                get(setup_gateway_tls_stream),
            )
            .route("/network/{network_id}/gateways", get(gateway_status))
            .route(
                "/network/{network_id}/gateways/{gateway_id}",
                put(change_gateway).delete(remove_gateway),
            )
            .route("/network/{network_id}/devices", post(add_user_devices))
            .route(
                "/network/{network_id}/device/{device_id}/config",
                get(download_config),
            )
            .route("/network/{network_id}/stats/users", get(devices_stats))
            .route("/network/{network_id}/stats", get(network_stats))
            .route(
                "/network/{location_id}/snat",
                get(list_snat_bindings).post(create_snat_binding),
            )
            .route(
                "/network/{location_id}/snat/{user_id}",
                put(modify_snat_binding).delete(delete_snat_binding),
            )
            .route("/outdated", get(outdated_components)),
    );

    let webapp = webapp.nest(
        "/api/v1/worker",
        Router::new()
            .route("/job", post(create_job))
            .route("/token", get(create_worker_token))
            .route("/", get(list_workers))
            .route("/{id}", delete(remove_worker).get(job_status))
            .layer(Extension(worker_state)),
    );

    let webapp = webapp.layer(DefguardVersionLayer::new(version)).layer(
        SetResponseHeaderLayer::if_not_present(
            headers::CONTENT_SECURITY_POLICY_HEADER_NAME,
            headers::CONTENT_SECURITY_POLICY_HEADER_VALUE,
        ),
    );

    let swagger =
        SwaggerUi::new("/api-docs").url("/api-docs/openapi.json", openapi::ApiDoc::openapi());

    webapp
        .with_state(AppState::new(
            pool.clone(),
            webhook_tx,
            webhook_rx,
            wireguard_tx,
            failed_logins,
            event_tx,
            incompatible_components,
            proxy_control_tx.clone(),
        ))
        .layer(Extension(pool))
        .layer(Extension(proxy_control_tx))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    info_span!(
                        "http_request",
                        method = ?request.method(),
                        path = ?request.uri(),
                    )
                })
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .merge(swagger)
}

/// Runs core web server exposing REST API.
#[instrument(skip_all)]
pub async fn run_web_server(
    worker_state: Arc<Mutex<WorkerState>>,
    webhook_tx: UnboundedSender<AppEvent>,
    webhook_rx: UnboundedReceiver<AppEvent>,
    wireguard_tx: Sender<GatewayEvent>,
    pool: PgPool,
    failed_logins: Arc<Mutex<FailedLoginMap>>,
    event_tx: UnboundedSender<ApiEvent>,
    incompatible_components: Arc<RwLock<IncompatibleComponents>>,
    proxy_control_tx: tokio::sync::mpsc::Sender<ProxyControlMessage>,
) -> Result<(), anyhow::Error> {
    let webapp = build_webapp(
        webhook_tx,
        webhook_rx,
        wireguard_tx,
        worker_state,
        pool,
        failed_logins,
        event_tx,
        Version::parse(VERSION)?,
        incompatible_components,
        proxy_control_tx,
    );
    info!("Started web services");
    let server_config = server_config();
    let addr = SocketAddr::new(
        server_config
            .http_bind_address
            .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
        server_config.http_port,
    );
    let listener = TcpListener::bind(&addr).await?;
    serve(
        listener,
        webapp.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .map_err(|err| anyhow!("Web server can't be started {err}"))
}

/// Automates test objects creation to easily setup development environment.
/// Test network keys:
/// Public: zGMeVGm9HV9I4wSKF9AXmYnnAIhDySyqLMuKpcfIaQo=
/// Private: MAk3d5KuB167G88HM7nGYR6ksnPMAOguAg2s5EcPp1M=
/// Test device keys:
/// Public: gQYL5eMeFDj0R+lpC7oZyIl0/sNVmQDC6ckP7husZjc=
/// Private: wGS1qdJfYbWJsOUuP1IDgaJYpR+VaKZPVZvdmLjsH2Y=
#[allow(deprecated)]
pub async fn init_dev_env(config: &DefGuardConfig) {
    info!("Initializing dev environment");
    let pool = init_db(
        &config.database_host,
        config.database_port,
        &config.database_name,
        &config.database_user,
        config.database_password.expose_secret(),
    )
    .await;

    // initialize admin user
    User::init_admin_user(&pool, config.default_admin_password.expose_secret())
        .await
        .expect("Failed to create admin user");

    let ca = CertificateAuthority::new("Defguard Dev", "defguard-dev@defguard.net", 5000)
        .expect("Failed to create CA");

    initialize_current_settings(&pool)
        .await
        .expect("Could not initialize current settings in the database");
    let mut settings = Settings::get_current_settings();
    settings.ca_cert_der = Some(ca.cert_der().to_vec());
    settings.ca_key_der = Some(ca.key_pair_der().to_vec());
    settings.ca_expiry = Some(ca.expiry().expect("Failed to get CA expiry"));
    settings.initial_setup_completed = true;
    // This should possibly be initialized somehow differently in the future since we are deprecating the enrollment URL env var.
    settings.public_proxy_url = config.enrollment_url.to_string();
    settings.defguard_url = config.url.to_string();
    update_current_settings(&pool, settings)
        .await
        .expect("Failed to update settings");

    let mut transaction = pool
        .begin()
        .await
        .expect("Failed to initialize transaction");

    let network = if let Some(networks) =
        WireguardNetwork::find_by_name(&mut *transaction, "TestNet")
            .await
            .expect("Failed to search for test network")
    {
        info!("Test network exists already, skipping creation...");
        networks.into_iter().next().unwrap()
    } else {
        info!("Creating test network");
        let mut network = WireguardNetwork::new(
            "TestNet".to_string(),
            vec![IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 1, 1, 1)), 24).unwrap()],
            50051,
            "0.0.0.0".to_string(),
            None,
            DEFAULT_WIREGUARD_MTU,
            0,
            vec![IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 1, 1, 0)), 24).unwrap()],
            DEFAULT_KEEPALIVE_INTERVAL,
            DEFAULT_DISCONNECT_THRESHOLD,
            false,
            false,
            LocationMfaMode::Disabled,
            ServiceLocationMode::Disabled,
        );
        network.pubkey = "zGMeVGm9HV9I4wSKF9AXmYnnAIhDySyqLMuKpcfIaQo=".to_string();
        network.prvkey = "MAk3d5KuB167G88HM7nGYR6ksnPMAOguAg2s5EcPp1M=".to_string();
        network
            .save(&mut *transaction)
            .await
            .expect("Could not save network")
    };

    if Device::find_by_pubkey(
        &mut *transaction,
        "gQYL5eMeFDj0R+lpC7oZyIl0/sNVmQDC6ckP7husZjc=",
    )
    .await
    .expect("Failed to search for test device")
    .is_some()
    {
        info!("Test device exists already, skipping creation...");
    } else {
        info!("Creating test device");
        let device = Device::new(
            "TestDevice".to_string(),
            "gQYL5eMeFDj0R+lpC7oZyIl0/sNVmQDC6ckP7husZjc=".to_string(),
            1,
            DeviceType::User,
            None,
            true,
        )
        .save(&mut *transaction)
        .await
        .expect("Could not save device");
        device
            .assign_next_network_ip(&mut transaction, &network, None, None)
            .await
            .expect("Could not assign IP to device");
    }

    for app_id in 1..=3 {
        OAuth2Client::new(
            vec![format!("https://app-{app_id}.com")],
            vec!["openid".into(), "profile".into(), "email".into()],
            format!("app-{app_id}"),
        )
        .save(&mut *transaction)
        .await
        .expect("Could not save oauth2client");
    }
    transaction
        .commit()
        .await
        .expect("Failed to commit transaction");

    info!("Dev environment initialized - TestNet, TestDevice, AuthorizedApps added");
}

/// Create a new VPN location.
/// Meant to be used to automate setting up a new defguard instance.
/// If the network ID has been specified, it will be assumed that the user wants to update the existing network or create a new one with a predefined ID.
/// This is mainly used for deployment purposes where the network ID must be known beforehand.
///
/// If there is no ID specified, the function will only create the network if no other network exists.
/// In other words, multiple networks can be created, but only if the ID is predefined for each network.
pub async fn init_vpn_location(
    pool: &PgPool,
    args: &InitVpnLocationArgs,
) -> Result<String, anyhow::Error> {
    // The ID is predefined
    let network = if let Some(location_id) = args.id {
        let mut transaction = pool.begin().await?;
        // If the network already exists, update it, assuming that's the user's intent.
        let network = if let Some(mut network) =
            WireguardNetwork::find_by_id(&mut *transaction, location_id).await?
        {
            network.name.clone_from(&args.name);
            network.address = vec![args.address];
            network.port = args.port;
            network.endpoint.clone_from(&args.endpoint);
            network.dns.clone_from(&args.dns);
            network.allowed_ips.clone_from(&args.allowed_ips);
            network.save(&mut *transaction).await?;
            sync_location_allowed_devices(&network, &mut transaction, None).await?;
            network
        }
        // Otherwise create it with the predefined ID
        else {
            let network = WireguardNetwork::new(
                args.name.clone(),
                vec![args.address],
                args.port,
                args.endpoint.clone(),
                args.dns.clone(),
                args.mtu as i32,
                i64::from(args.fwmark),
                args.allowed_ips.clone(),
                DEFAULT_KEEPALIVE_INTERVAL,
                DEFAULT_DISCONNECT_THRESHOLD,
                false,
                false,
                LocationMfaMode::Disabled,
                ServiceLocationMode::Disabled,
            )
            .save(&mut *transaction)
            .await?;
            if network.id != location_id {
                return Err(anyhow!(
                    "Failed to initialize VPN location. The ID of the newly created network ({}) does not match \
                    the predefined ID ({location_id}). The predefined ID must be the next available ID.",
                    network.id
                ));
            }
            network.add_all_allowed_devices(&mut transaction).await?;
            network
        };
        transaction.commit().await?;
        network
    }
    // No predefined ID, add the network if no other networks are present
    else {
        // check if a VPN location exists already
        let networks = WireguardNetwork::all(pool).await?;
        if !networks.is_empty() {
            return Err(anyhow!(
                "Failed to initialize first VPN location. Location already exists."
            ));
        }

        // create a new network
        WireguardNetwork::new(
            args.name.clone(),
            vec![args.address],
            args.port,
            args.endpoint.clone(),
            args.dns.clone(),
            args.mtu as i32,
            i64::from(args.fwmark),
            args.allowed_ips.clone(),
            DEFAULT_KEEPALIVE_INTERVAL,
            DEFAULT_DISCONNECT_THRESHOLD,
            false,
            false,
            LocationMfaMode::Disabled,
            ServiceLocationMode::Disabled,
        )
        .save(pool)
        .await?
    };

    // generate gateway token
    let token = Claims::new(
        ClaimsType::Gateway,
        format!("DEFGUARD-NETWORK-{}", network.id),
        network.id.to_string(),
        u32::MAX.into(),
    )
    .to_jwt()?;

    Ok(token)
}

pub fn is_valid_phone_number(number: &str) -> bool {
    PHONE_NUMBER_REGEX.is_match(number)
}

#[cfg(test)]
mod test {

    use super::is_valid_phone_number;

    #[test]
    fn test_is_valid_phone_number_dg25_10() {
        let valid_numbers = &[
            "+48 (91) 123-456",
            "123 456 7890",
            "+1 (202) 555-0173",
            "91-1234-5678",
            "(22) 567 890",
        ];
        for number in valid_numbers {
            assert!(is_valid_phone_number(number));
        }

        let invalid_numbers = &[
            "4*4",
            "+48  123456789",
            "123-456-789-0000",
            "(+48) 123 456",
            "202.555.0173",
            "(12345) 6789",
            "+48 (91) 123-456 000 111",
        ];
        for number in invalid_numbers {
            assert!(!is_valid_phone_number(number));
        }
    }
}
