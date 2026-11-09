use std::net::IpAddr;

use chrono::NaiveDateTime;
use model_derive::Model;
use sqlx::{PgExecutor, Type, query};

use crate::db::{Id, NoId};

#[derive(Clone, Copy, Debug, PartialEq, Type)]
#[sqlx(type_name = "connection_direction", rename_all = "lowercase")]
pub enum ConnectionDirection {
    Inbound,
    Outbound,
}

#[derive(Clone, Copy, Debug, PartialEq, Type)]
#[sqlx(type_name = "connection_status", rename_all = "lowercase")]
pub enum ConnectionStatus {
    Allowed,
    Blocked,
}

/// Traffic flow reported by a Gateway.
#[derive(Clone, Debug, Model)]
#[table(connection_event)]
pub struct ConnectionEvent<I = NoId> {
    pub id: I,
    pub location_id: Id,
    /// Not a foreign key, so events outlive deleted Gateways.
    pub gateway_id: Id,
    pub timestamp: NaiveDateTime,
    #[model(enum)]
    pub direction: ConnectionDirection,
    #[model(enum)]
    pub status: ConnectionStatus,
    /// IANA protocol number (e.g. 6 for TCP, 17 for UDP).
    pub protocol: i32,
    #[model(ip)]
    pub source_ip: IpAddr,
    pub source_port: Option<i32>,
    #[model(ip)]
    pub destination_ip: IpAddr,
    pub destination_port: Option<i32>,
    /// ACL rule that matched, if any.
    pub rule_id: Option<Id>,
    /// Bytes sent by the VPN device.
    pub upload_bytes: i64,
    /// Bytes received by the VPN device.
    pub download_bytes: i64,
    /// Number of packets of this kind (aggregation).
    pub count: i64,
}

impl ConnectionEvent<Id> {
    /// Delete events older than `threshold`. Returns the number of removed rows.
    pub async fn delete_older_than<'e, E>(
        executor: E,
        threshold: NaiveDateTime,
    ) -> sqlx::Result<u64>
    where
        E: PgExecutor<'e>,
    {
        let result = query!(
            "DELETE FROM connection_event WHERE timestamp < $1",
            threshold
        )
        .execute(executor)
        .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeDelta, Utc};
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

    use super::*;
    use crate::db::{models::WireguardNetwork, setup_pool};

    fn build_event(location_id: Id, timestamp: NaiveDateTime) -> ConnectionEvent {
        ConnectionEvent {
            id: NoId,
            location_id,
            gateway_id: 1,
            timestamp,
            direction: ConnectionDirection::Inbound,
            status: ConnectionStatus::Allowed,
            protocol: 17,
            source_ip: "10.0.0.2".parse().unwrap(),
            source_port: Some(5353),
            destination_ip: "fd00::1".parse().unwrap(),
            destination_port: None,
            rule_id: None,
            upload_bytes: 1,
            download_bytes: 2,
            count: 1,
        }
    }

    #[sqlx::test]
    async fn test_save_and_delete_older_than(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let location = WireguardNetwork::default().save(&pool).await.unwrap();

        let now = Utc::now().naive_utc();
        build_event(location.id, now - TimeDelta::days(10))
            .save(&pool)
            .await
            .unwrap();
        build_event(location.id, now).save(&pool).await.unwrap();

        let count = ConnectionEvent::count(&pool).await.unwrap();
        assert_eq!(count, 2);

        let removed = ConnectionEvent::delete_older_than(&pool, now - TimeDelta::days(1))
            .await
            .unwrap();
        assert_eq!(removed, 1);

        let count = ConnectionEvent::count(&pool).await.unwrap();
        assert_eq!(count, 1);
    }
}
