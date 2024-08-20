use model_derive::Model;
use sqlx::PgExecutor;
use struct_patch::Patch;

use crate::enterprise::license::{get_cached_license, validate_license};

#[derive(Model, Deserialize, Serialize, Patch, Default)]
#[patch(attribute(derive(Serialize, Deserialize)))]
pub struct EnterpriseSettings {
    pub id: Option<i64>,
    // If true, only admins can manage devices
    pub disable_device_management: bool,
}

impl EnterpriseSettings {
    /// If license is valid returns current [`EnterpriseSettings`] object.
    /// Otherwise returns [`EnterpriseSettings::default()`].
    pub async fn get<'e, E>(executor: E) -> Result<Self, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
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
