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
    /// VPN device taking part in the connection; see `save_all_with_device`.
    pub device_id: Option<Id>,
    /// Bytes sent by the VPN device.
    pub upload_bytes: i64,
    /// Bytes received by the VPN device.
    pub download_bytes: i64,
    /// Number of packets of this kind (aggregation).
    pub count: i64,
}

impl ConnectionEvent<NoId> {
    /// Stores `events` in one statement, setting `device_id` to the VPN device with an event's
    /// address, preferring the source of outbound and the destination of inbound traffic.
    pub async fn save_all_with_device<'e, E>(executor: E, events: &[Self]) -> sqlx::Result<u64>
    where
        E: PgExecutor<'e>,
    {
        if events.is_empty() {
            return Ok(0);
        }
        let location_id: Vec<_> = events.iter().map(|e| e.location_id).collect();
        let gateway_id: Vec<_> = events.iter().map(|e| e.gateway_id).collect();
        let timestamp: Vec<_> = events.iter().map(|e| e.timestamp).collect();
        let direction: Vec<_> = events.iter().map(|e| e.direction).collect();
        let status: Vec<_> = events.iter().map(|e| e.status).collect();
        let protocol: Vec<_> = events.iter().map(|e| e.protocol).collect();
        let source_ip: Vec<_> = events.iter().map(|e| e.source_ip).collect();
        let source_port: Vec<_> = events.iter().map(|e| e.source_port).collect();
        let destination_ip: Vec<_> = events.iter().map(|e| e.destination_ip).collect();
        let destination_port: Vec<_> = events.iter().map(|e| e.destination_port).collect();
        let rule_id: Vec<_> = events.iter().map(|e| e.rule_id).collect();
        let upload_bytes: Vec<_> = events.iter().map(|e| e.upload_bytes).collect();
        let download_bytes: Vec<_> = events.iter().map(|e| e.download_bytes).collect();
        let count: Vec<_> = events.iter().map(|e| e.count).collect();

        // Addresses of devices in the batch's locations are looked up once, then joined.
        let result = query!(
            "WITH e AS (SELECT * FROM UNNEST($1::bigint[], $2::bigint[], $3::timestamp[], \
            $4::connection_direction[], $5::connection_status[], $6::int[], $7::inet[], \
            $8::int[], $9::inet[], $10::int[], $11::bigint[], $12::bigint[], $13::bigint[], \
            $14::bigint[]) AS e(location_id, gateway_id, timestamp, direction, status, protocol, \
            source_ip, source_port, destination_ip, destination_port, rule_id, upload_bytes, \
            download_bytes, count)), \
            ips AS (SELECT DISTINCT ON (wnd.wireguard_network_id, ip) \
            wnd.wireguard_network_id location_id, ip, wnd.device_id \
            FROM wireguard_network_device wnd, UNNEST(wnd.wireguard_ips) ip \
            WHERE wnd.wireguard_network_id IN (SELECT location_id FROM e)) \
            INSERT INTO connection_event (location_id, gateway_id, timestamp, direction, status, \
            protocol, source_ip, source_port, destination_ip, destination_port, rule_id, \
            device_id, upload_bytes, download_bytes, count) \
            SELECT e.location_id, e.gateway_id, e.timestamp, e.direction, e.status, e.protocol, \
            e.source_ip, e.source_port, e.destination_ip, e.destination_port, e.rule_id, \
            CASE WHEN e.direction = 'outbound' THEN COALESCE(src.device_id, dst.device_id) \
            ELSE COALESCE(dst.device_id, src.device_id) END, \
            e.upload_bytes, e.download_bytes, e.count \
            FROM e \
            LEFT JOIN ips src ON src.location_id = e.location_id AND src.ip = e.source_ip \
            LEFT JOIN ips dst ON dst.location_id = e.location_id AND dst.ip = e.destination_ip",
            &location_id,
            &gateway_id,
            &timestamp,
            &direction as &[ConnectionDirection],
            &status as &[ConnectionStatus],
            &protocol,
            &source_ip as &[IpAddr],
            &source_port as &[Option<i32>],
            &destination_ip as &[IpAddr],
            &destination_port as &[Option<i32>],
            &rule_id as &[Option<Id>],
            &upload_bytes,
            &download_bytes,
            &count,
        )
        .execute(executor)
        .await?;

        Ok(result.rows_affected())
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
