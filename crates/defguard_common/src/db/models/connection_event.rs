use std::net::IpAddr;

use chrono::NaiveDateTime;
use model_derive::Model;
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, Type, query};

use crate::db::{Id, NoId};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, Type)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "connection_direction", rename_all = "lowercase")]
pub enum ConnectionDirection {
    Inbound,
    Outbound,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, Type)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
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
    /// VPN device taking part in the connection; see `save_with_device`.
    pub device_id: Option<Id>,
    /// Bytes sent by the VPN device.
    pub upload_bytes: i64,
    /// Bytes received by the VPN device.
    pub download_bytes: i64,
    /// Number of packets of this kind (aggregation).
    pub count: i64,
}

impl ConnectionEvent<NoId> {
    /// Like `save`, but also sets `device_id` to the VPN device with one of the event's addresses
    /// in its location, preferring the source of outbound and the destination of inbound traffic.
    pub async fn save_with_device<'e, E>(self, executor: E) -> sqlx::Result<ConnectionEvent<Id>>
    where
        E: PgExecutor<'e>,
    {
        let (device_ip, peer_ip) = match self.direction {
            ConnectionDirection::Outbound => (self.source_ip, self.destination_ip),
            ConnectionDirection::Inbound => (self.destination_ip, self.source_ip),
        };
        let row = query!(
            "INSERT INTO connection_event (location_id, gateway_id, timestamp, direction, status, \
            protocol, source_ip, source_port, destination_ip, destination_port, rule_id, \
            device_id, upload_bytes, download_bytes, count) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, \
            (SELECT device_id FROM wireguard_network_device \
            WHERE wireguard_network_id = $1 \
            AND ($12 = ANY(wireguard_ips) OR $13 = ANY(wireguard_ips)) \
            ORDER BY $12 = ANY(wireguard_ips) DESC LIMIT 1), \
            $14, $15, $16) \
            RETURNING id, device_id",
            self.location_id,
            self.gateway_id,
            self.timestamp,
            &self.direction as &ConnectionDirection,
            &self.status as &ConnectionStatus,
            self.protocol,
            &self.source_ip as &IpAddr,
            self.source_port,
            &self.destination_ip as &IpAddr,
            self.destination_port,
            self.rule_id,
            &device_ip as &IpAddr,
            &peer_ip as &IpAddr,
            self.upload_bytes,
            self.download_bytes,
            self.count,
        )
        .fetch_one(executor)
        .await?;

        Ok(ConnectionEvent {
            device_id: row.device_id,
            ..self
        }
        .with_id(row.id))
    }
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
mod tests;
