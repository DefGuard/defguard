use model_derive::Model;
use sqlx::PgExecutor;
use struct_patch::Patch;

use crate::enterprise::license::{get_cached_license, validate_license};

#[derive(Debug, Model, Deserialize, Serialize, Patch)]
#[patch(attribute(derive(Serialize, Deserialize)))]
pub struct EnterpriseSettings {
    #[serde(skip)]
    pub id: Option<i64>,
    // If true, only admins can manage devices
    pub admin_device_management: bool,
    // If true, the option to route all traffic through the vpn is disabled in the client
    pub disable_all_traffic: bool,
    // If true, manual WireGuard setup is disabled
    pub only_client_activation: bool,
}

// We want to be conscious of what the defaults are here
#[allow(clippy::derivable_impls)]
impl Default for EnterpriseSettings {
    fn default() -> Self {
        Self {
            id: None,
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
        let is_valid = {
            let license = get_cached_license();
            validate_license(license.as_ref()).is_ok()
        };
        if is_valid {
            let settings = Self::find_by_id(executor, 1).await?;
            Ok(settings.expect("EnterpriseSettings not found"))
        } else {
            Ok(EnterpriseSettings::default())
        }
    }
}
