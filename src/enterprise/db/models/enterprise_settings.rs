use sqlx::{query, query_as, PgExecutor};
use struct_patch::Patch;

use crate::enterprise::is_enterprise_enabled;

#[derive(Debug, Deserialize, Patch, Serialize)]
#[patch(attribute(derive(Deserialize, Serialize)))]
pub struct EnterpriseSettings {
    // If true, only admins can manage devices
    pub admin_device_management: bool,
    // If true, the option to route all traffic through the vpn is disabled in the client
    pub disable_all_traffic: bool,
    // If true, manual WireGuard setup is disabled
    pub only_client_activation: bool,
}

// We want to be conscious of what the defaults are here
impl Default for EnterpriseSettings {
    fn default() -> Self {
        Self {
            admin_device_management: false,
            disable_all_traffic: false,
            only_client_activation: false,
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
        if is_enterprise_enabled() {
            let settings = query_as!(
                Self,
                "SELECT admin_device_management, \
                disable_all_traffic, only_client_activation \
                FROM \"enterprisesettings\" WHERE id = 1",
            )
            .fetch_optional(executor)
            .await?;
            Ok(settings.expect("EnterpriseSettings not found"))
        } else {
            Ok(EnterpriseSettings::default())
        }
    }

    pub(crate) async fn save<'e, E>(&self, executor: E) -> Result<(), sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "UPDATE \"enterprisesettings\" SET \
            admin_device_management = $1, \
            disable_all_traffic = $2, \
            only_client_activation = $3 \
            WHERE id = 1",
            self.admin_device_management,
            self.disable_all_traffic,
            self.only_client_activation,
        )
        .execute(executor)
        .await?;

        Ok(())
    }
}
