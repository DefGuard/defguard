use std::net::IpAddr;

use chrono::NaiveDateTime;
use defguard_common::db::Id;

#[derive(Debug)]
pub struct SessionManagerEvent {
    pub context: SessionManagerEventContext,
    pub event: SessionManagerEventType,
}

#[derive(Debug)]
pub struct SessionManagerEventContext {
    pub timestamp: NaiveDateTime,
    pub user_id: Id,
    pub username: String,
    pub public_ip: IpAddr,
    pub device_id: Id,
    pub device_name: String,
}

#[derive(Debug)]
pub enum SessionManagerEventType {
    ClientConnected,
    ClientDisconnected,
    MfaClientConnected,
    MfaClientDisconnected,
}
