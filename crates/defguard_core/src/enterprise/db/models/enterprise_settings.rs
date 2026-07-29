use defguard_common::db::{Id, models::Settings};
use sqlx::{PgExecutor, Type, query, query_as};
use struct_patch::Patch;

use crate::enterprise::is_business_license_active;

#[derive(Clone, Debug, Deserialize, PartialEq, Patch, Serialize)]
#[patch(attribute(derive(Deserialize, Serialize)))]
pub struct EnterpriseSettings {
    /// If true, only admins can manage devices
    pub admin_device_management: bool,
    /// Describes allowed routing options for clients connecting to the instance.
    pub client_traffic_policy: ClientTrafficPolicy,
    /// If true, manual WireGuard setup is disabled
    pub only_client_activation: bool,
    /// If true, bare WireGuard tunnels are disabled in the desktop client and CLI.
    pub disable_tunnels: bool,
    /// If true, the client download page is shown during enrollment.
    pub display_download_step: bool,
    /// If true, the password reset option is displayed on the Edge home page.
    pub display_password_reset: bool,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct GroupClientTrafficPolicies {
    pub none: Vec<Id>,
    pub disable_all_traffic: Vec<Id>,
    pub force_all_traffic: Vec<Id>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct EnterpriseSettingsInfo {
    #[serde(flatten)]
    pub settings: EnterpriseSettings,
    pub group_client_traffic_policies: GroupClientTrafficPolicies,
}

impl EnterpriseSettingsInfo {
    #[must_use]
    pub fn new(
        settings: EnterpriseSettings,
        group_client_traffic_policies: GroupClientTrafficPolicies,
    ) -> Self {
        Self {
            settings,
            group_client_traffic_policies,
        }
    }
}

// We want to be conscious of what the defaults are here
#[allow(clippy::derivable_impls)]
impl Default for EnterpriseSettings {
    fn default() -> Self {
        Self {
            admin_device_management: false,
            client_traffic_policy: ClientTrafficPolicy::default(),
            only_client_activation: false,
            disable_tunnels: false,
            display_download_step: true,
            display_password_reset: true,
        }
    }
}

impl EnterpriseSettings {
    /// If license is valid returns current [`EnterpriseSettings`] object.
    /// Otherwise returns [`EnterpriseSettings::default()`].
    pub async fn get<'e, E>(executor: E) -> Result<Self, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        // avoid holding the rwlock across await, makes the future !Send
        // and therefore unusable in axum handlers
        if is_business_license_active() {
            let settings = query_as!(
                Self,
                "SELECT admin_device_management, \
				client_traffic_policy \"client_traffic_policy: ClientTrafficPolicy\", \
				only_client_activation, \
				disable_tunnels, \
				display_download_step, \
				display_password_reset \
                FROM \"enterprisesettings\" WHERE id = 1",
            )
            .fetch_optional(executor)
            .await?;
            Ok(settings.expect("EnterpriseSettings not found"))
        } else {
            Ok(Self::default())
        }
    }

    /// The effective password-reset visibility for Edge components.
    /// Password reset requires email delivery, so if SMTP is missing the
    /// option should not appear even when the admin toggle is on.
    #[must_use]
    pub fn edge_can_display_password_reset(&self) -> bool {
        let settings = Settings::get_current_settings();
        self.display_password_reset && settings.smtp_configured()
    }

    pub(crate) async fn save<'e, E>(&self, executor: E) -> Result<(), sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "UPDATE \"enterprisesettings\" SET \
            admin_device_management = $1, \
			client_traffic_policy = $2, \
            only_client_activation = $3, \
            disable_tunnels = $4, \
            display_download_step = $5, \
            display_password_reset = $6 \
            WHERE id = 1",
            self.admin_device_management,
            self.client_traffic_policy as ClientTrafficPolicy,
            self.only_client_activation,
            self.disable_tunnels,
            self.display_download_step,
            self.display_password_reset,
        )
        .execute(executor)
        .await?;

        Ok(())
    }
}

/// Describes allowed traffic options for clients connecting to the instance.
#[derive(Clone, Deserialize, Serialize, PartialEq, Type, Debug, Default, Copy)]
#[sqlx(type_name = "client_traffic_policy", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ClientTrafficPolicy {
    /// No restrictions
    #[default]
    None,
    /// Clients are not allowed to route all traffic through the VPN.
    DisableAllTraffic,
    /// Clients are forced to route all traffic through the VPN.
    ForceAllTraffic,
}

/// Resolves group policies over the instance-level policy.
///
/// A configured group policy takes precedence over the instance policy. When a user belongs to
/// multiple groups, disabling all traffic takes precedence over forcing all traffic. An explicit
/// `None` group policy takes precedence over the instance policy when no restrictive group policy
/// is present.
#[must_use]
pub fn resolve_client_traffic_policy(
    instance_policy: ClientTrafficPolicy,
    group_policies: impl IntoIterator<Item = ClientTrafficPolicy>,
) -> ClientTrafficPolicy {
    let mut has_force_all_traffic = false;
    let mut has_none = false;

    for policy in group_policies {
        match policy {
            ClientTrafficPolicy::DisableAllTraffic => {
                return ClientTrafficPolicy::DisableAllTraffic;
            }
            ClientTrafficPolicy::ForceAllTraffic => has_force_all_traffic = true,
            ClientTrafficPolicy::None => has_none = true,
        }
    }

    if has_force_all_traffic {
        ClientTrafficPolicy::ForceAllTraffic
    } else if has_none {
        ClientTrafficPolicy::None
    } else {
        instance_policy
    }
}

#[cfg(test)]
mod tests {
    use super::{ClientTrafficPolicy, resolve_client_traffic_policy};

    #[test]
    fn instance_policy_is_used_without_group_overrides() {
        assert_eq!(
            resolve_client_traffic_policy(ClientTrafficPolicy::ForceAllTraffic, []),
            ClientTrafficPolicy::ForceAllTraffic
        );
    }

    #[test]
    fn group_policy_overrides_instance_policy() {
        assert_eq!(
            resolve_client_traffic_policy(
                ClientTrafficPolicy::DisableAllTraffic,
                [ClientTrafficPolicy::ForceAllTraffic]
            ),
            ClientTrafficPolicy::ForceAllTraffic
        );
    }

    #[test]
    fn disable_all_traffic_wins_conflicting_group_policies() {
        assert_eq!(
            resolve_client_traffic_policy(
                ClientTrafficPolicy::ForceAllTraffic,
                [
                    ClientTrafficPolicy::ForceAllTraffic,
                    ClientTrafficPolicy::DisableAllTraffic,
                ]
            ),
            ClientTrafficPolicy::DisableAllTraffic
        );
    }

    #[test]
    fn explicit_none_group_policy_overrides_instance_policy() {
        assert_eq!(
            resolve_client_traffic_policy(
                ClientTrafficPolicy::ForceAllTraffic,
                [ClientTrafficPolicy::None]
            ),
            ClientTrafficPolicy::None
        );
    }
}
