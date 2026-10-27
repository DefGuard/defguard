use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, query, query_as};

use crate::db::Id;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, sqlx::Type)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[sqlx(type_name = "os_type", rename_all = "lowercase")]
pub enum OsType {
    Windows,
    Macos,
    Linux,
    Ios,
    Android,
}

impl OsType {
    /// Parses an OS name as clients report it, ignoring case.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "windows" => Some(Self::Windows),
            "macos" | "darwin" => Some(Self::Macos),
            "linux" => Some(Self::Linux),
            "ios" => Some(Self::Ios),
            "android" => Some(Self::Android),
            _ => None,
        }
    }
}

/// What the client app last reported about itself and its operating system.
#[derive(Debug, PartialEq)]
pub struct DeviceClientInfo {
    pub device_id: Id,
    pub client_version: Option<String>,
    pub os_type: OsType,
    pub os_version: String,
    pub os_edition: Option<String>,
}

impl DeviceClientInfo {
    /// Clients report on every poll, so an unchanged report skips the write.
    pub async fn upsert<'e, E>(&self, executor: E) -> sqlx::Result<()>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "INSERT INTO device_client_info AS d \
            (device_id, client_version, os_type, os_version, os_edition) \
            VALUES ($1, $2, $3, $4, $5) \
            ON CONFLICT (device_id) DO UPDATE SET client_version = EXCLUDED.client_version, \
            os_type = EXCLUDED.os_type, os_version = EXCLUDED.os_version, \
            os_edition = EXCLUDED.os_edition \
            WHERE (d.client_version, d.os_type, d.os_version, d.os_edition) \
            IS DISTINCT FROM (EXCLUDED.client_version, EXCLUDED.os_type, EXCLUDED.os_version, \
            EXCLUDED.os_edition)",
            self.device_id,
            self.client_version,
            &self.os_type as &OsType,
            self.os_version,
            self.os_edition,
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    pub async fn find_by_device_id<'e, E>(executor: E, device_id: Id) -> sqlx::Result<Option<Self>>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT device_id, client_version, os_type \"os_type: OsType\", os_version, \
            os_edition \
            FROM device_client_info WHERE device_id = $1",
            device_id
        )
        .fetch_optional(executor)
        .await
    }
}

#[cfg(test)]
mod tests {
    use sqlx::{
        PgPool,
        postgres::{PgConnectOptions, PgPoolOptions},
        query_scalar,
    };

    use super::{DeviceClientInfo, OsType};
    use crate::db::{
        Id,
        models::{Device, DeviceType, User},
        setup_pool,
    };

    async fn row_version(pool: &PgPool, device_id: Id) -> String {
        query_scalar!(
            "SELECT xmin::text \"xmin!\" FROM device_client_info WHERE device_id = $1",
            device_id
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[sqlx::test]
    async fn test_upsert_writes_only_changed_reports(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let user = User::new(
            "testuser",
            None,
            "LastName",
            "FirstName",
            "test@example.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();
        let device = Device::new(
            "testdevice".to_owned(),
            "testpubkey".to_owned(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        let mut report = DeviceClientInfo {
            device_id: device.id,
            client_version: Some("2.3.0".to_owned()),
            os_type: OsType::Linux,
            os_version: "24.4.0".to_owned(),
            os_edition: None,
        };
        report.upsert(&pool).await.unwrap();
        let first_write = row_version(&pool, device.id).await;

        report.upsert(&pool).await.unwrap();
        assert_eq!(row_version(&pool, device.id).await, first_write);

        report.os_version = "26.4.0".to_owned();
        report.upsert(&pool).await.unwrap();
        assert_ne!(row_version(&pool, device.id).await, first_write);
        assert_eq!(
            DeviceClientInfo::find_by_device_id(&pool, device.id)
                .await
                .unwrap(),
            Some(report)
        );
    }
}
