use defguard_common::db::{Id, NoId};
use model_derive::Model;
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, query, query_as};
use utoipa::ToSchema;

/// Device posture check policy. Defines the security requirements a client
/// device must satisfy before being allowed to connect to an assigned VPN location.
#[derive(Clone, Debug, Deserialize, Model, Serialize, ToSchema, PartialEq)]
#[table(device_posture)]
pub struct DevicePosture<I = NoId> {
    pub id: I,
    pub name: String,
    pub description: Option<String>,
    pub min_client_version: Option<String>,
    pub allow_prerelease_client: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, sqlx::Type)]
#[sqlx(type_name = "os_type", rename_all = "lowercase")]
pub enum OsType {
    Windows,
    Macos,
    Linux,
    Ios,
    Android,
}

/// Per-OS security rule row belonging to a [`DevicePosture`] policy.
#[derive(Clone, Debug, Deserialize, Model, Serialize, ToSchema, PartialEq)]
#[table(device_posture_os_rule)]
pub struct DevicePostureOsRule<I = NoId> {
    pub id: I,
    pub posture_id: Id,
    #[model(enum)]
    pub os_type: OsType,
    // shared
    pub min_os_version: Option<String>,
    // windows, macos, linux
    pub disk_encryption_required: Option<bool>,
    // windows only
    pub antivirus_required: Option<bool>,
    pub ad_domain_joined_required: Option<bool>,
    pub windows_security_update_current: Option<bool>,
    // linux only
    pub min_kernel_version: Option<String>,
    // macos, android only
    pub device_integrity_required: Option<bool>,
}

impl DevicePostureOsRule<Id> {
    /// Returns all OS rules belonging to the given posture policy.
    pub async fn find_by_posture<'e, E: PgExecutor<'e>>(
        executor: E,
        posture_id: Id,
    ) -> sqlx::Result<Vec<Self>> {
        query_as!(
            Self,
            r#"SELECT id, posture_id,
               os_type AS "os_type: OsType",
               min_os_version, disk_encryption_required,
               antivirus_required, ad_domain_joined_required,
               windows_security_update_current, min_kernel_version,
               device_integrity_required
               FROM device_posture_os_rule WHERE posture_id = $1"#,
            posture_id
        )
        .fetch_all(executor)
        .await
    }

    /// Deletes all OS rules for a given posture policy.
    /// Used when replacing the full rule set on update.
    pub async fn delete_by_posture<'e, E: PgExecutor<'e>>(
        executor: E,
        posture_id: Id,
    ) -> sqlx::Result<()> {
        query!(
            "DELETE FROM device_posture_os_rule WHERE posture_id = $1",
            posture_id
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}
