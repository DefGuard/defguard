use defguard_common::{
    db::{
        Id,
        models::{
            Settings, User, WireguardNetwork,
            mfa_flow::{LocationMfaFlowItem, MfaFlow},
            vpn_client_session::VpnClientMfaMethod,
            wireguard::ServiceLocationMode,
        },
    },
    types::UrlParseError,
};
use reqwest::Url;
use sqlx::{PgExecutor, PgPool};

use crate::enterprise::{
    LicenseFeature,
    db::models::{
        enterprise_settings::{
            ClientTrafficPolicy, EnterpriseSettings, resolve_client_traffic_policy,
        },
        group_client_traffic_policy::GroupClientTrafficPolicy,
        openid_provider::{OpenIdProvider, OpenIdProviderKind},
    },
    has_enterprise_access, is_business_license_active, is_oidc_mfa_available,
    license::LicenseTier,
    licensed_mfa_setup_methods,
};

pub mod client_version;
pub mod proxy;
pub mod utils;

pub mod proto {
    pub mod enterprise {
        pub mod license {
            tonic::include_proto!("enterprise.license");
        }
    }
}

use defguard_proto::client_types::{
    MfaCapabilities, MfaMethod, MfaUserState, OpenIdProviderKind as ProtoOpenIdProviderKind,
};

#[derive(Debug)]
pub struct InstanceInfo {
    id: uuid::Uuid,
    name: String,
    url: Url,
    proxy_url: Url,
    username: String,
    client_traffic_policy: ClientTrafficPolicy,
    enterprise_enabled: bool,
    openid_display_name: Option<String>,
    openid_provider_kind: Option<OpenIdProviderKind>,
    disable_tunnels: bool,
    smtp_configured: bool,
    openid_available: bool,
    pub configured_methods: Vec<VpnClientMfaMethod>,
}

#[derive(Debug, thiserror::Error)]
/// Errors that can occur while building client instance information.
pub enum InstanceInfoBuildError {
    #[error("failed to load enterprise settings: {0}")]
    Database(#[from] sqlx::Error),
    #[error("failed to parse instance URL: {0}")]
    UrlParse(#[from] UrlParseError),
}

impl InstanceInfo {
    /// Builds client instance information with the effective user traffic policy.
    pub async fn build(
        pool: &PgPool,
        settings: &Settings,
        user: &User<Id>,
        openid_provider: Option<OpenIdProvider<Id>>,
        device_id: Option<Id>,
    ) -> Result<Self, InstanceInfoBuildError> {
        let enterprise_settings = EnterpriseSettings::get(pool).await?;
        let smtp_configured = settings.smtp_configured();
        let oidc_configured = is_oidc_mfa_available(openid_provider.is_some());
        let mut configured_methods = Vec::with_capacity(VpnClientMfaMethod::ALL.len());
        for method in VpnClientMfaMethod::ALL {
            if method
                .is_configured(pool, user, device_id, smtp_configured, oidc_configured)
                .await?
            {
                configured_methods.push(method);
            }
        }
        let client_traffic_policy = if is_business_license_active() {
            let group_policies = GroupClientTrafficPolicy::find_by_user_id(pool, user.id)
                .await?
                .into_iter()
                .map(|policy| policy.client_traffic_policy);
            resolve_client_traffic_policy(enterprise_settings.client_traffic_policy, group_policies)
        } else {
            enterprise_settings.client_traffic_policy
        };
        let openid_display_name = openid_provider
            .as_ref()
            .map(|provider| provider.display_name.clone())
            .unwrap_or_default();
        let openid_provider_kind = openid_provider.map(|provider| provider.kind);
        let url = Settings::url()?;
        let proxy_url = settings.proxy_public_url()?;
        Ok(Self {
            id: settings.uuid,
            name: settings.instance_name.clone(),
            url,
            proxy_url,
            username: user.username.clone(),
            client_traffic_policy,
            enterprise_enabled: is_business_license_active(),
            openid_display_name,
            openid_provider_kind,
            disable_tunnels: enterprise_settings.disable_tunnels,
            smtp_configured,
            openid_available: oidc_configured,
            configured_methods,
        })
    }
}

impl From<InstanceInfo> for defguard_proto::client_types::InstanceInfo {
    fn from(instance: InstanceInfo) -> Self {
        Self {
            name: instance.name,
            id: instance.id.to_string(),
            url: instance.url.to_string(),
            proxy_url: instance.proxy_url.to_string(),
            username: instance.username,
            // Ensure backwards compatibility.
            #[allow(deprecated)]
            disable_all_traffic: instance.client_traffic_policy
                == ClientTrafficPolicy::DisableAllTraffic,
            client_traffic_policy: Some(instance.client_traffic_policy as i32),
            enterprise_enabled: instance.enterprise_enabled,
            openid_display_name: instance.openid_display_name,
            openid_provider_kind: instance
                .openid_provider_kind
                .map(|kind| ProtoOpenIdProviderKind::from(&kind) as i32),
            disable_tunnels: Some(instance.disable_tunnels),
            smtp_configured: Some(instance.smtp_configured),
            openid_available: Some(instance.openid_available),
            mfa_user_state: Some(MfaUserState {
                configured_methods: instance
                    .configured_methods
                    .into_iter()
                    .map(|method| MfaMethod::from(method) as i32)
                    .collect(),
            }),
            mfa_capabilities: Some(MfaCapabilities {
                setup_methods: licensed_mfa_setup_methods()
                    .into_iter()
                    .map(|method| MfaMethod::from(method) as i32)
                    .collect(),
                authorize_methods: VpnClientMfaMethod::CONFIG_AUTHORIZE
                    .map(|method| MfaMethod::from(method) as i32)
                    .to_vec(),
            }),
        }
    }
}

pub use defguard_common::gateway_event::{
    GatewayCommand, send_gateway_command, send_multiple_gateway_commands,
};

/// If this location is marked as a service location, checks if all requirements are met for it to
/// function:
/// - Enterprise is enabled
#[must_use]
pub fn should_prevent_service_location_usage(location: &WireguardNetwork<Id>) -> bool {
    location.service_location_mode != ServiceLocationMode::Disabled
        && !has_enterprise_access(Some(LicenseFeature::ServiceLocations))
}

/// Returns the minimum license tier required by the assignments. `None` means they fit the Free
/// tier.
///
/// Keep this logic in sync with [`WireguardNetwork::validate_mfa_flow_assignments_license`].
#[must_use]
fn required_assignment_tier(items: &[LocationMfaFlowItem]) -> Option<LicenseTier> {
    if items.is_empty() {
        return None;
    }
    if items.iter().any(|item| !item.group_ids.is_empty()) {
        return Some(LicenseTier::Enterprise);
    }
    if items.len() > 1 || items[0].step_count > 1 {
        return Some(LicenseTier::Business);
    }
    None
}

/// Returns the minimum license tier required by the location's saved MFA assignments. `None` means
/// MFA is disabled or the assignments fit the Free tier.
///
/// Assignments persist while MFA is disabled, so check the flag before loading them.
pub async fn location_mfa_required_tier<'e, E: PgExecutor<'e>>(
    executor: E,
    location: &WireguardNetwork<Id>,
) -> sqlx::Result<Option<LicenseTier>> {
    if !location.mfa_enabled {
        return Ok(None);
    }
    let items = MfaFlow::for_location(executor, location.id).await?;
    Ok(required_assignment_tier(&items))
}

/// Returns whether the location's MFA policy is unavailable under the active license.
///
/// A saved policy can outlive a license change. Skip the location when the active license can no
/// longer enforce it.
pub async fn should_prevent_mfa_location_usage<'e, E: PgExecutor<'e>>(
    executor: E,
    location: &WireguardNetwork<Id>,
) -> sqlx::Result<bool> {
    Ok(
        match location_mfa_required_tier(executor, location).await? {
            None => false,
            Some(LicenseTier::Business) => !is_business_license_active(),
            Some(LicenseTier::Enterprise) => !has_enterprise_access(None),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(step_count: i64, group_ids: Vec<Id>) -> LocationMfaFlowItem {
        LocationMfaFlowItem {
            id: 1,
            title: "FlowTitle".into(),
            step_count,
            group_ids,
            group_names: Vec::new(),
            position: 0,
            is_default: true,
        }
    }

    #[test]
    fn test_required_assignment_tier() {
        assert_eq!(required_assignment_tier(&[]), None);
        assert_eq!(required_assignment_tier(&[item(1, Vec::new())]), None);
        assert_eq!(
            required_assignment_tier(&[item(2, Vec::new())]),
            Some(LicenseTier::Business)
        );
        assert_eq!(
            required_assignment_tier(&[item(1, Vec::new()), item(1, Vec::new())]),
            Some(LicenseTier::Business)
        );
        assert_eq!(
            required_assignment_tier(&[item(1, Vec::new()), item(1, vec![7])]),
            Some(LicenseTier::Enterprise)
        );
    }
}
