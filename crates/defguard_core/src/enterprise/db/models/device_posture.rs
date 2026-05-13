use defguard_common::db::{Id, NoId};
use model_derive::Model;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgExecutor, query, query_as, query_scalar};
use utoipa::ToSchema;

/// Device posture check policy. Defines the security requirements a client
/// device must satisfy before being allowed to connect to an assigned VPN location.
#[derive(Clone, Debug, Deserialize, FromRow, Model, Serialize, ToSchema, PartialEq)]
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
    // Windows, macOS, Linux
    pub disk_encryption_required: Option<bool>,
    // Windows only
    pub antivirus_required: Option<bool>,
    pub ad_domain_joined_required: Option<bool>,
    pub windows_security_update_current: Option<bool>,
    // Linux only
    pub min_kernel_version: Option<String>,
    // macOS, iOS, Android only
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
            "SELECT id, posture_id, os_type \"os_type: OsType\", min_os_version, \
            disk_encryption_required, antivirus_required, ad_domain_joined_required, \
            windows_security_update_current, min_kernel_version, \
            device_integrity_required \
            FROM device_posture_os_rule WHERE posture_id = $1",
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

/// A point-in-time snapshot of a posture check policy and its related data,
/// used as the payload for audit events.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DevicePostureSnapshot {
    pub device_posture: DevicePosture<Id>,
    pub os_rules: Vec<DevicePostureOsRule<Id>>,
    pub location_ids: Vec<Id>,
}

/// Join table row linking a posture check policy to a VPN location.
pub struct DevicePostureLocation {
    pub posture_id: Id,
    pub location_id: Id,
}

impl DevicePostureLocation {
    /// Replaces all posture assignments for a location with the given list.
    /// Returns the resulting set of posture IDs.
    pub async fn set_for_location(
        conn: &mut sqlx::PgConnection,
        location_id: Id,
        posture_ids: &[Id],
    ) -> sqlx::Result<Vec<Id>> {
        query!(
            "DELETE FROM device_posture_location WHERE location_id = $1",
            location_id
        )
        .execute(&mut *conn)
        .await?;

        query!(
            "INSERT INTO device_posture_location (posture_id, location_id) \
             SELECT unnest($1::bigint[]), $2",
            posture_ids,
            location_id
        )
        .execute(&mut *conn)
        .await?;

        Ok(posture_ids.to_vec())
    }

    /// Replaces all location assignments for a posture with the given list.
    /// Returns the resulting set of location IDs.
    pub async fn set_for_posture(
        conn: &mut sqlx::PgConnection,
        posture_id: Id,
        location_ids: &[Id],
    ) -> sqlx::Result<Vec<Id>> {
        query!(
            "DELETE FROM device_posture_location WHERE posture_id = $1",
            posture_id
        )
        .execute(&mut *conn)
        .await?;

        query!(
            "INSERT INTO device_posture_location (posture_id, location_id) \
             SELECT $1, unnest($2::bigint[])",
            posture_id,
            location_ids
        )
        .execute(&mut *conn)
        .await?;

        Ok(location_ids.to_vec())
    }

    /// Returns the IDs of all locations assigned to the given posture.
    pub async fn find_by_posture<'e, E>(executor: E, posture_id: Id) -> sqlx::Result<Vec<Id>>
    where
        E: PgExecutor<'e>,
    {
        query_scalar!(
            "SELECT location_id FROM device_posture_location WHERE posture_id = $1",
            posture_id
        )
        .fetch_all(executor)
        .await
    }

    /// Returns the IDs of all postures assigned to the given location.
    pub async fn find_by_location<'e, E>(executor: E, location_id: Id) -> sqlx::Result<Vec<Id>>
    where
        E: PgExecutor<'e>,
    {
        query_scalar!(
            "SELECT posture_id FROM device_posture_location WHERE location_id = $1",
            location_id
        )
        .fetch_all(executor)
        .await
    }

    /// Returns true if the given location has at least one posture check assigned.
    pub async fn location_has_postures<'e, E>(executor: E, location_id: Id) -> sqlx::Result<bool>
    where
        E: PgExecutor<'e>,
    {
        let exists = query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM device_posture_location WHERE location_id = $1)",
            location_id
        )
        .fetch_one(executor)
        .await?;
        Ok(exists.unwrap_or(false))
    }
}
