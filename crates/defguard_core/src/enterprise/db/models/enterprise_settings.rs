use sqlx::{PgExecutor, Type, query, query_as};
use struct_patch::Patch;

use crate::enterprise::is_business_license_active;

#[derive(Debug, Deserialize, Patch, Serialize)]
#[patch(attribute(derive(Deserialize, Serialize)))]
pub struct EnterpriseSettings {
    /// If true, only admins can manage devices
    pub admin_device_management: bool,
    /// Describes allowed routing options for clients connecting to the instance.
    pub client_traffic_policy: ClientTrafficPolicy,
    /// If true, the client download page is shown during enrollment.
    pub display_download_step: bool,
    /// If true, manual WireGuard setup is disabled
    pub only_client_activation: bool,
    /// If true, the password reset option is displayed on the Edge home page.
    pub display_password_reset: bool,
}

// We want to be conscious of what the defaults are here
#[allow(clippy::derivable_impls)]
impl Default for EnterpriseSettings {
    fn default() -> Self {
        Self {
            admin_device_management: false,
            client_traffic_policy: ClientTrafficPolicy::default(),
            display_download_step: true,
            only_client_activation: false,
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
				display_download_step, \
				only_client_activation, \
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

    pub(crate) async fn save<'e, E>(&self, executor: E) -> Result<(), sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "UPDATE \"enterprisesettings\" SET \
            admin_device_management = $1, \
			client_traffic_policy = $2, \
            display_download_step = $3, \
            only_client_activation = $4, \
            display_password_reset = $5 \
            WHERE id = 1",
            self.admin_device_management,
            self.client_traffic_policy as ClientTrafficPolicy,
            self.display_download_step,
            self.only_client_activation,
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
