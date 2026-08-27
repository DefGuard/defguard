use std::{net::IpAddr, str::FromStr};

use defguard_common::{
    csv::AsCsv,
    db::{
        Id,
        models::{
            Device, DeviceType, Settings, User, WireguardNetwork,
            device::{DeviceConfig, WireguardNetworkDevice},
            mfa_flow::MfaFlowStep,
            vpn_client_session::VpnClientMfaMethod,
            wireguard::{LocationMfaMode, ServiceLocationMode},
        },
    },
};
use defguard_proto::{
    client_types::{
        DeviceConfig as ProtoDeviceConfig, DeviceConfigResponse,
        LocationMfaMode as ProtoLocationMfaMode, MfaMethod, MfaStep, MfaStepMethod,
    },
    proxy::DeviceInfo,
};
use sqlx::PgPool;
use tonic::Status;

use super::InstanceInfo;
use crate::{
    device_access::build_device_config,
    enterprise::{db::models::openid_provider::OpenIdProvider, is_business_license_active},
    grpc::{
        client_version::{ClientFeature, should_omit_location_for_device},
        should_prevent_service_location_usage,
    },
};

pub async fn build_device_config_response(
    pool: &PgPool,
    device: Device<Id>,
    token: Option<String>,
    device_info: Option<DeviceInfo>,
) -> Result<DeviceConfigResponse, Status> {
    let settings = Settings::get_current_settings();

    let openid_provider = OpenIdProvider::get_current(pool).await.map_err(|err| {
        error!("Failed to get OpenID provider: {err}");
        Status::internal(format!("unexpected error: {err}"))
    })?;

    let smtp_configured = settings.smtp_configured();
    let oidc_configured = is_business_license_active() && openid_provider.is_some();

    let locations = WireguardNetwork::all(pool).await.map_err(|err| {
        error!("Failed to fetch all networks: {err}");
        Status::internal(format!("unexpected error: {err}"))
    })?;

    let mut configs = Vec::new();
    let user = User::find_by_id(pool, device.user_id)
        .await
        .map_err(|_| {
            error!("Failed to get user: {}", device.user_id);
            Status::internal("unexpected error")
        })?
        .ok_or_else(|| {
            error!("User not found: {}", device.user_id);
            Status::internal("unexpected error")
        })?;
    if device.device_type == DeviceType::Network {
        let wireguard_network_device = WireguardNetworkDevice::find_first(pool, device.id)
            .await
            .map_err(|err| {
                error!(
                    "Failed to fetch WireGuard network device for device {}: {err}",
                    device.id
                );
                Status::internal(format!("unexpected error: {err}"))
            })?;
        if let Some(wireguard_network_device) = wireguard_network_device {
            let location = wireguard_network_device
                .network(pool)
                .await
                .map_err(|err| {
                    error!(
                        "Failed to fetch network for WireGuard network device {}: {err}",
                        device.name
                    );
                    Status::internal(format!("unexpected error: {err}"))
                })?;

            if location.service_location_mode != ServiceLocationMode::Disabled {
                error!(
                    "Network device {} tried to fetch config for service location {}, which is unsupported.",
                    device.name, location.name
                );
                return Err(Status::permission_denied(
                    "service location mode is not available for network devices",
                ));
            }

            let mut conn = pool.acquire().await.map_err(|err| {
                error!("Failed to acquire connection: {err}");
                Status::internal(format!("unexpected error: {err}"))
            })?;

            let device_config =
                build_device_config(&mut conn, &location, &wireguard_network_device, &user)
                    .await
                    .map_err(|err| {
                        error!("Failed to build device config: {err}");
                        Status::internal(format!("unexpected error: {err}"))
                    })?;

            if device_config.mfa_enabled {
                error!(
                    "Network device {} tried to fetch config for location {} with MFA enabled, which is unsupported.",
                    device.name, location.name
                );
                return Err(Status::failed_precondition(
                    "network devices cannot connect to locations with MFA enabled",
                ));
            }

            let config = to_wire_device_config(
                pool,
                device_config,
                &user,
                device.id,
                smtp_configured,
                oidc_configured,
                false,
            )
            .await?;
            configs.push(config);
        }
    } else {
        let supports_multi_step_mfa =
            ClientFeature::MultiStepMfa.is_supported_by_device(device_info.as_ref());
        for location in locations {
            let wireguard_network_device = WireguardNetworkDevice::find(
                pool,
                device.id,
                location.id,
            )
            .await
            .map_err(|err| {
                error!(
                    "Failed to fetch WireGuard network device for device {} and network {}: {err}",
                    device.id, location.id
                );
                Status::internal(format!("unexpected error: {err}"))
            })?;
            if should_prevent_service_location_usage(&location) {
                warn!(
                    "Tried to use service location {} with disabled enterprise features.",
                    location.name
                );
                continue;
            }
            if location.service_location_mode != ServiceLocationMode::Disabled
                && !ClientFeature::ServiceLocations.is_supported_by_device(device_info.as_ref())
            {
                info!(
                    "Device {} does not support service locations feature, skipping sending network {} configuration to device {}.",
                    device.name, location.name, device.name
                );
                continue;
            }
            if let Some(wireguard_network_device) = wireguard_network_device {
                let mut conn = pool.acquire().await.map_err(|err| {
                    error!("Failed to acquire connection: {err}");
                    Status::internal(format!("unexpected error: {err}"))
                })?;

                let device_config =
                    build_device_config(&mut conn, &location, &wireguard_network_device, &user)
                        .await
                        .map_err(|err| {
                            error!("Failed to build device config: {err}");
                            Status::internal(format!("unexpected error: {err}"))
                        })?;

                if device_config.posture_check_required
                    && !ClientFeature::PostureChecks.is_supported_by_device(device_info.as_ref())
                {
                    info!(
                        "Device {} does not support posture checks feature, skipping sending network {} configuration to device {}.",
                        device.name, location.name, device.name
                    );
                    continue;
                }

                if should_omit_location_for_device(
                    device_config.location_mfa_mode.clone(),
                    device_info.as_ref(),
                ) {
                    info!(
                        "Device {} does not support multi-step MFA, skipping sending network {} configuration to device {}.",
                        device.name, location.name, device.name
                    );
                    continue;
                }

                let config = to_wire_device_config(
                    pool,
                    device_config,
                    &user,
                    device.id,
                    smtp_configured,
                    oidc_configured,
                    supports_multi_step_mfa,
                )
                .await?;
                configs.push(config);
            }
        }
    }

    info!(
        "User {}({}) device {}({}) automatically fetched the newest configuration.",
        user.username, user.id, device.name, device.id
    );

    let instance_info = InstanceInfo::build(pool, &settings, &user, openid_provider)
        .await
        .map_err(|err| {
            error!("Failed to build instance info: {err}");
            Status::internal(format!("unexpected error: {err}"))
        })?;

    Ok(DeviceConfigResponse {
        device: Some(device.into()),
        configs,
        instance: Some(instance_info.into()),
        token,
    })
}

/// Maps the resolved MFA flow steps to the wire `MfaStep` list, computing each method's
/// `configured` flag for the given user/device.
pub async fn build_wire_steps(
    pool: &PgPool,
    steps: &[MfaFlowStep<Id>],
    user: &User<Id>,
    device_id: Id,
    smtp_configured: bool,
    oidc_configured: bool,
) -> Result<Vec<MfaStep>, Status> {
    let mut wire_steps = Vec::with_capacity(steps.len());
    for step in steps {
        let mut methods = Vec::with_capacity(step.methods.len());
        for &method in &step.methods {
            let configured = method
                .is_configured(pool, user, device_id, smtp_configured, oidc_configured)
                .await
                .map_err(|err| {
                    error!("Failed to compute MFA method configuration: {err}");
                    Status::internal("unexpected error")
                })?;
            methods.push(MfaStepMethod {
                method: <VpnClientMfaMethod as Into<MfaMethod>>::into(method) as i32,
                configured,
            });
        }
        wire_steps.push(MfaStep { methods });
    }
    Ok(wire_steps)
}

/// Computes the wire `steps` for a location, applying per-client capability branching and the
/// fail-closed empty-flow guard. Returns the resolved flow for clients that support multi-step MFA
/// and an empty list for legacy clients. Rejects an MFA-enabled location that has no resolvable
/// flow, so it is never advertised to a capable client as `steps = []`.
pub async fn wire_steps_for_device(
    pool: &PgPool,
    location_mfa_mode_is_none: bool,
    resolved_steps: &[MfaFlowStep<Id>],
    network_name: &str,
    user: &User<Id>,
    device_id: Id,
    smtp_configured: bool,
    oidc_configured: bool,
    supports_multi_step_mfa: bool,
) -> Result<Vec<MfaStep>, Status> {
    if location_mfa_mode_is_none && resolved_steps.is_empty() {
        return Err(Status::failed_precondition(format!(
            "location {network_name} has MFA enabled but no MFA flow is configured"
        )));
    }
    if supports_multi_step_mfa {
        build_wire_steps(
            pool,
            resolved_steps,
            user,
            device_id,
            smtp_configured,
            oidc_configured,
        )
        .await
    } else {
        Ok(Vec::new())
    }
}

/// Builds a wire `DeviceConfig` from the resolved internal `DeviceConfig`, applying the per-client
/// capability branching (`steps`) and the fail-closed empty-flow guard. This is the single
/// conversion point: the dumb field mapping and the `configured` computation both live here,
/// because `configured` requires the user/device in scope.
pub async fn to_wire_device_config(
    pool: &PgPool,
    device_config: DeviceConfig,
    user: &User<Id>,
    device_id: Id,
    smtp_configured: bool,
    oidc_configured: bool,
    supports_multi_step_mfa: bool,
) -> Result<ProtoDeviceConfig, Status> {
    let steps = wire_steps_for_device(
        pool,
        device_config.location_mfa_mode.is_none(),
        &device_config.steps,
        &device_config.network_name,
        user,
        device_id,
        smtp_configured,
        oidc_configured,
        supports_multi_step_mfa,
    )
    .await?;

    Ok(ProtoDeviceConfig {
        config: device_config.config,
        network_id: device_config.network_id,
        network_name: device_config.network_name,
        assigned_ip: device_config.address.as_csv(),
        endpoint: device_config.endpoint,
        pubkey: device_config.pubkey,
        allowed_ips: device_config.allowed_ips.as_csv(),
        dns: device_config.dns,
        keepalive_interval: device_config.keepalive_interval,
        #[allow(deprecated)]
        mfa_enabled: device_config.mfa_enabled,
        #[allow(deprecated)]
        location_mfa_mode: device_config
            .location_mfa_mode
            .map(|mode| <LocationMfaMode as Into<ProtoLocationMfaMode>>::into(mode).into()),
        service_location_mode: Some(
            <ServiceLocationMode as Into<defguard_proto::client_types::ServiceLocationMode>>::into(
                device_config.service_location_mode,
            )
            .into(),
        ),
        posture_check_required: Some(device_config.posture_check_required),
        steps,
    })
}

/// Parses `DeviceInfo` returning client IP address and user agent.
pub fn parse_client_ip_agent(info: &Option<DeviceInfo>) -> Result<(IpAddr, String), String> {
    let Some(info) = info else {
        error!("Missing DeviceInfo in proxy request");
        return Err("missing device info".to_owned());
    };

    let ip = IpAddr::from_str(&info.ip_address).map_err(|_| {
        let msg = format!("invalid IP address: {}", info.ip_address);
        error!(msg);
        msg
    })?;
    let user_agent = info.user_agent.clone().unwrap_or_else(String::new);
    let escaped_agent = tera::escape_html(&user_agent);

    Ok((ip, escaped_agent))
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use defguard_common::db::{
        Id,
        models::{
            Device, DeviceType, Settings, User, WireguardNetwork,
            biometric_auth::BiometricAuth,
            device::WireguardNetworkDevice,
            mfa_flow::{LocationMfaFlowAssignment, MfaFlow},
            settings::{initialize_current_settings, update_current_settings},
            vpn_client_session::VpnClientMfaMethod,
            wireguard::ServiceLocationMode,
        },
        setup_pool,
    };
    use defguard_proto::{
        client_types::{LocationMfaMode, MfaMethod},
        proxy::DeviceInfo,
    };
    use ipnetwork::IpNetwork;
    use sqlx::{
        PgPool,
        postgres::{PgConnectOptions, PgPoolOptions},
    };
    use tonic::Code;

    use super::{build_device_config_response, build_wire_steps};
    use crate::enterprise::license::{
        License, LicenseTier, SupportType, get_cached_license, set_cached_license,
    };

    const DEFGUARD_URL: &str = "http://localhost:8000";
    const PROXY_URL: &str = "http://localhost:8080";

    async fn init_settings(pool: &PgPool) {
        initialize_current_settings(pool)
            .await
            .expect("failed to init settings");
        let mut settings = Settings::get_current_settings();
        settings.defguard_url = DEFGUARD_URL.to_owned();
        settings.public_proxy_url = PROXY_URL.to_owned();
        update_current_settings(pool, settings)
            .await
            .expect("failed to update settings");
    }

    async fn create_user(pool: &PgPool) -> User<Id> {
        User::new(
            "mfa-gate-test",
            Some("pass123"),
            "Tester",
            "MfaGate",
            "mfa-gate@example.com",
            None,
        )
        .save(pool)
        .await
        .expect("failed to create user")
    }

    async fn create_device(pool: &PgPool, user_id: Id) -> Device<Id> {
        Device::new(
            "mfa-gate-device".to_owned(),
            "mfa-gate-pubkey".to_owned(),
            user_id,
            DeviceType::User,
            None,
            true,
        )
        .save(pool)
        .await
        .expect("failed to create device")
    }

    async fn create_network_device(
        pool: &PgPool,
        user_id: Id,
        name: &str,
        pubkey: &str,
    ) -> Device<Id> {
        Device::new(
            name.to_owned(),
            pubkey.to_owned(),
            user_id,
            DeviceType::Network,
            None,
            true,
        )
        .save(pool)
        .await
        .expect("failed to create network device")
    }

    /// Creates an MFA-enabled location with a single default flow built from `step_methods`.
    async fn create_location(
        pool: &PgPool,
        name: &str,
        address_octet: u8,
        step_methods: Vec<Vec<VpnClientMfaMethod>>,
    ) -> WireguardNetwork<Id> {
        let network = WireguardNetwork::new(
            name.to_owned(),
            51820,
            "vpn.example.com".to_owned(),
            None,
            [IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap()],
            true,
            false,
            false,
            false,
            true, // mfa_enabled
            ServiceLocationMode::Disabled,
        )
        .set_address([
            IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 10, address_octet, 1)), 24).unwrap(),
        ])
        .expect("failed to set location address")
        .save(pool)
        .await
        .expect("failed to create location");

        let mut tx = pool.begin().await.expect("failed to begin tx");
        let (flow, _) = MfaFlow::create(&mut tx, format!("{name}-flow"), step_methods)
            .await
            .expect("failed to create flow");
        MfaFlow::assign_to_location(
            &mut tx,
            network.id,
            &[LocationMfaFlowAssignment {
                flow_id: flow.id,
                is_default: true,
                group_ids: Vec::new(),
            }],
        )
        .await
        .expect("failed to assign flow");
        tx.commit().await.expect("failed to commit tx");

        network
    }

    /// Creates an MFA-disabled location with no flow configured.
    async fn create_disabled_location(
        pool: &PgPool,
        name: &str,
        address_octet: u8,
    ) -> WireguardNetwork<Id> {
        WireguardNetwork::new(
            name.to_owned(),
            51820,
            "vpn.example.com".to_owned(),
            None,
            [IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap()],
            true,
            false,
            false,
            false,
            false, // mfa_enabled
            ServiceLocationMode::Disabled,
        )
        .set_address([
            IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 10, address_octet, 1)), 24).unwrap(),
        ])
        .expect("failed to set location address")
        .save(pool)
        .await
        .expect("failed to create location")
    }

    /// Creates an MFA-enabled location with no flow assigned (the transient
    /// "MFA on, no policy built yet" state).
    async fn create_mfa_location_without_flow(
        pool: &PgPool,
        name: &str,
        address_octet: u8,
    ) -> WireguardNetwork<Id> {
        WireguardNetwork::new(
            name.to_owned(),
            51820,
            "vpn.example.com".to_owned(),
            None,
            [IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap()],
            true,
            false,
            false,
            false,
            true, // mfa_enabled
            ServiceLocationMode::Disabled,
        )
        .set_address([
            IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 10, address_octet, 1)), 24).unwrap(),
        ])
        .expect("failed to set location address")
        .save(pool)
        .await
        .expect("failed to create location")
    }

    async fn attach_device(pool: &PgPool, location_id: Id, device_id: Id) {
        WireguardNetworkDevice::new(
            location_id,
            device_id,
            vec![IpAddr::V4(Ipv4Addr::new(10, 10, 0, 10))],
        )
        .insert(pool)
        .await
        .expect("failed to attach device");
    }

    fn device_info(version: &str) -> Option<DeviceInfo> {
        Some(DeviceInfo {
            version: Some(version.to_owned()),
            ..Default::default()
        })
    }

    /// Builds a valid Business-tier license for tests that exercise licensed behavior.
    ///
    /// Setting this mutates the process-global license cache, so callers save and restore it
    /// around their body. The restore is best-effort: a parallel test that also mutates the cache
    /// can still race.
    fn business_license() -> License {
        License {
            customer_id: "test".to_owned(),
            subscription: false,
            valid_until: None,
            limits: None,
            version_date_limit: None,
            tier: LicenseTier::Business,
            support_type: SupportType::Basic,
            features: Vec::new(),
        }
    }

    #[sqlx::test]
    async fn test_multi_step_location_omitted_for_legacy_client(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        init_settings(&pool).await;
        let saved_license = get_cached_license().clone();
        set_cached_license(Some(business_license()));
        let user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;

        // Multi-step location: two steps, so `derive_legacy_mode` returns `None`.
        let multi_step = create_location(
            &pool,
            "multi-step-location",
            1,
            vec![
                vec![VpnClientMfaMethod::Totp],
                vec![VpnClientMfaMethod::Email],
            ],
        )
        .await;
        // Legacy-derivable location: single internal step, so `Some(Internal)`.
        let internal = create_location(
            &pool,
            "internal-location",
            2,
            vec![vec![
                VpnClientMfaMethod::Totp,
                VpnClientMfaMethod::Email,
                VpnClientMfaMethod::Biometric,
                VpnClientMfaMethod::MobileApprove,
            ]],
        )
        .await;
        attach_device(&pool, multi_step.id, device.id).await;
        attach_device(&pool, internal.id, device.id).await;

        // Legacy client (2.1.0): the multi-step location is omitted, the internal one retained.
        let response =
            build_device_config_response(&pool, device.clone(), None, device_info("2.1.0"))
                .await
                .expect("failed to build config for legacy client");
        let names: Vec<&str> = response
            .configs
            .iter()
            .map(|config| config.network_name.as_str())
            .collect();
        assert!(
            !names.contains(&"multi-step-location"),
            "multi-step location must be omitted for a legacy client, got: {names:?}"
        );
        assert!(
            names.contains(&"internal-location"),
            "legacy-derivable location must be retained, got: {names:?}"
        );

        // Capable client (2.2.0): both locations are retained.
        let response =
            build_device_config_response(&pool, device.clone(), None, device_info("2.2.0"))
                .await
                .expect("failed to build config for capable client");
        let names: Vec<&str> = response
            .configs
            .iter()
            .map(|config| config.network_name.as_str())
            .collect();
        assert!(
            names.contains(&"multi-step-location"),
            "multi-step location must be retained for a capable client, got: {names:?}"
        );
        assert!(
            names.contains(&"internal-location"),
            "legacy-derivable location must be retained, got: {names:?}"
        );

        set_cached_license(saved_license);
    }

    #[sqlx::test]
    async fn test_network_device_rejected_on_mfa_locations(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        init_settings(&pool).await;
        let user = create_user(&pool).await;

        // Multi-step location: a network device must be rejected.
        let multi_step = create_location(
            &pool,
            "multi-step-location",
            1,
            vec![
                vec![VpnClientMfaMethod::Totp],
                vec![VpnClientMfaMethod::Email],
            ],
        )
        .await;
        let multi_step_device =
            create_network_device(&pool, user.id, "multi-step-device", "multi-step-pubkey").await;
        attach_device(&pool, multi_step.id, multi_step_device.id).await;

        let error = build_device_config_response(&pool, multi_step_device, None, None)
            .await
            .err()
            .expect("network device on multi-step location should be rejected");
        assert_eq!(error.code(), Code::FailedPrecondition);
        assert_eq!(
            error.message(),
            "network devices cannot connect to locations with MFA enabled"
        );

        // Legacy-derivable (internal) location: also rejected, since a network device cannot
        // perform any MFA.
        let internal = create_location(
            &pool,
            "internal-location",
            2,
            vec![vec![
                VpnClientMfaMethod::Totp,
                VpnClientMfaMethod::Email,
                VpnClientMfaMethod::Biometric,
                VpnClientMfaMethod::MobileApprove,
            ]],
        )
        .await;
        let internal_device =
            create_network_device(&pool, user.id, "internal-device", "internal-pubkey").await;
        attach_device(&pool, internal.id, internal_device.id).await;

        let error = build_device_config_response(&pool, internal_device, None, None)
            .await
            .err()
            .expect("network device on legacy-derivable location should be rejected");
        assert_eq!(error.code(), Code::FailedPrecondition);
        assert_eq!(
            error.message(),
            "network devices cannot connect to locations with MFA enabled"
        );

        // MFA-disabled location: the config is still produced.
        let disabled = create_disabled_location(&pool, "disabled-location", 3).await;
        let disabled_device =
            create_network_device(&pool, user.id, "disabled-device", "disabled-pubkey").await;
        attach_device(&pool, disabled.id, disabled_device.id).await;

        let response = build_device_config_response(&pool, disabled_device, None, None)
            .await
            .expect("network device on MFA-disabled location should get config");
        assert_eq!(response.configs.len(), 1);
        assert_eq!(response.configs[0].network_name, "disabled-location");
    }

    /// Asserts every cell of the per-client-version config-shape matrix: four location
    /// configurations across a legacy (2.1.0) and a capable (2.2.0) client.
    #[sqlx::test]
    #[allow(deprecated)]
    async fn test_device_config_matrix(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        init_settings(&pool).await;
        let saved_license = get_cached_license().clone();
        set_cached_license(Some(business_license()));
        let user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;

        let disabled = create_disabled_location(&pool, "disabled-location", 1).await;
        let internal = create_location(
            &pool,
            "internal-location",
            2,
            vec![vec![
                VpnClientMfaMethod::Totp,
                VpnClientMfaMethod::Email,
                VpnClientMfaMethod::Biometric,
                VpnClientMfaMethod::MobileApprove,
            ]],
        )
        .await;
        let external = create_location(
            &pool,
            "external-location",
            3,
            vec![vec![VpnClientMfaMethod::Oidc]],
        )
        .await;
        let multi_step = create_location(
            &pool,
            "multi-step-location",
            4,
            vec![
                vec![VpnClientMfaMethod::Totp],
                vec![VpnClientMfaMethod::Email],
            ],
        )
        .await;

        attach_device(&pool, disabled.id, device.id).await;
        attach_device(&pool, internal.id, device.id).await;
        attach_device(&pool, external.id, device.id).await;
        attach_device(&pool, multi_step.id, device.id).await;

        // Legacy client (2.1.0): multi-step location omitted; others carry `location_mfa_mode`
        // and no `steps`.
        let response =
            build_device_config_response(&pool, device.clone(), None, device_info("2.1.0"))
                .await
                .expect("failed to build config for legacy client");
        let names: Vec<&str> = response
            .configs
            .iter()
            .map(|config| config.network_name.as_str())
            .collect();
        assert!(
            !names.contains(&"multi-step-location"),
            "multi-step location must be omitted for a legacy client, got: {names:?}"
        );

        let config = response
            .configs
            .iter()
            .find(|config| config.network_name == "disabled-location")
            .expect("disabled location must be present");
        assert!(!config.mfa_enabled);
        assert_eq!(
            config.location_mfa_mode,
            Some(LocationMfaMode::Disabled as i32)
        );
        assert!(config.steps.is_empty());

        let config = response
            .configs
            .iter()
            .find(|config| config.network_name == "internal-location")
            .expect("internal location must be present");
        assert!(config.mfa_enabled);
        assert_eq!(
            config.location_mfa_mode,
            Some(LocationMfaMode::Internal as i32)
        );
        assert!(config.steps.is_empty());

        let config = response
            .configs
            .iter()
            .find(|config| config.network_name == "external-location")
            .expect("external location must be present");
        assert!(config.mfa_enabled);
        assert_eq!(
            config.location_mfa_mode,
            Some(LocationMfaMode::External as i32)
        );
        assert!(config.steps.is_empty());

        // Capable client (2.2.0): every location present; `location_mfa_mode` carries the
        // derived value (same as legacy) and `steps` is populated from the resolved flow.
        let response =
            build_device_config_response(&pool, device.clone(), None, device_info("2.2.0"))
                .await
                .expect("failed to build config for capable client");

        let config = response
            .configs
            .iter()
            .find(|config| config.network_name == "disabled-location")
            .expect("disabled location must be present");
        assert!(!config.mfa_enabled);
        assert_eq!(
            config.location_mfa_mode,
            Some(LocationMfaMode::Disabled as i32)
        );
        assert!(config.steps.is_empty());

        let config = response
            .configs
            .iter()
            .find(|config| config.network_name == "internal-location")
            .expect("internal location must be present");
        assert!(config.mfa_enabled);
        assert_eq!(
            config.location_mfa_mode,
            Some(LocationMfaMode::Internal as i32)
        );
        assert_eq!(config.steps.len(), 1);
        assert_eq!(config.steps[0].methods.len(), 4);

        let config = response
            .configs
            .iter()
            .find(|config| config.network_name == "external-location")
            .expect("external location must be present");
        assert!(config.mfa_enabled);
        assert_eq!(
            config.location_mfa_mode,
            Some(LocationMfaMode::External as i32)
        );
        assert_eq!(config.steps.len(), 1);
        assert_eq!(config.steps[0].methods.len(), 1);

        let config = response
            .configs
            .iter()
            .find(|config| config.network_name == "multi-step-location")
            .expect("multi-step location must be present for a capable client");
        assert!(config.mfa_enabled);
        assert_eq!(config.location_mfa_mode, None);
        assert_eq!(config.steps.len(), 2);
        assert_eq!(config.steps[0].methods.len(), 1);
        assert_eq!(config.steps[1].methods.len(), 1);

        set_cached_license(saved_license);
    }

    /// An MFA-enabled location with no resolvable flow must fail closed for a capable client:
    /// the config poll is rejected rather than advertising `steps = []`.
    #[sqlx::test]
    async fn test_mfa_enabled_location_without_flow_is_rejected(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        init_settings(&pool).await;
        let saved_license = get_cached_license().clone();
        set_cached_license(Some(business_license()));
        let user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;

        let no_flow = create_mfa_location_without_flow(&pool, "no-flow-location", 1).await;
        attach_device(&pool, no_flow.id, device.id).await;

        // Capable client (2.2.0): the no-flow location must be rejected.
        let error = build_device_config_response(&pool, device, None, device_info("2.2.0"))
            .await
            .err()
            .expect("MFA-enabled location without a flow must be rejected");
        assert_eq!(error.code(), Code::FailedPrecondition);
        assert!(
            error.message().contains("no MFA flow is configured"),
            "unexpected message: {}",
            error.message()
        );

        set_cached_license(saved_license);
    }

    /// The wire `steps` carry each method's `configured` flag for the user's own setup, not just
    /// the method list: setup state AND deployment availability gate it.
    #[sqlx::test]
    async fn test_build_wire_steps_computes_configured_flags(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        init_settings(&pool).await;

        let mut user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;

        // User setup: TOTP enabled, email not set up, OIDC identity present, biometric
        // registered on this device (which also makes mobile-approve available).
        user.totp_enabled = true;
        user.openid_sub = Some("oidc-sub".to_owned());
        BiometricAuth::new(device.id, "biometric-pubkey".to_owned())
            .save(&pool)
            .await
            .expect("failed to save biometric auth");

        // A two-step flow covering all five methods.
        let mut tx = pool.begin().await.expect("failed to begin tx");
        let (_, steps) = MfaFlow::create(
            &mut tx,
            "wire-steps-flow".to_owned(),
            vec![
                vec![VpnClientMfaMethod::Totp, VpnClientMfaMethod::Email],
                vec![
                    VpnClientMfaMethod::Oidc,
                    VpnClientMfaMethod::Biometric,
                    VpnClientMfaMethod::MobileApprove,
                ],
            ],
        )
        .await
        .expect("failed to create flow");
        tx.commit().await.expect("failed to commit tx");

        // SMTP is configured, OIDC is not: email is gated on the user (not set up), OIDC on
        // deployment (no provider).
        let wire = build_wire_steps(&pool, &steps, &user, device.id, true, false)
            .await
            .expect("failed to build wire steps");

        assert_eq!(wire.len(), 2);

        assert_eq!(wire[0].methods.len(), 2);
        assert_eq!(wire[0].methods[0].method, MfaMethod::Totp as i32);
        assert!(wire[0].methods[0].configured);
        assert_eq!(wire[0].methods[1].method, MfaMethod::Email as i32);
        assert!(!wire[0].methods[1].configured);

        assert_eq!(wire[1].methods.len(), 3);
        assert_eq!(wire[1].methods[0].method, MfaMethod::Oidc as i32);
        assert!(!wire[1].methods[0].configured);
        assert_eq!(wire[1].methods[1].method, MfaMethod::Biometric as i32);
        assert!(wire[1].methods[1].configured);
        assert_eq!(wire[1].methods[2].method, MfaMethod::MobileApprove as i32);
        assert!(wire[1].methods[2].configured);
    }
}
