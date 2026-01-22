use std::net::IpAddr;

use chrono::NaiveDateTime;
use defguard_common::db::{
    Id,
    models::{Device, User, WireguardNetwork},
};

#[derive(Debug)]
pub struct SessionManagerEvent {
    pub context: SessionManagerEventContext,
    pub event: SessionManagerEventType,
}

#[derive(Debug)]
pub struct SessionManagerEventContext {
    pub timestamp: NaiveDateTime,
    pub location: WireguardNetwork<Id>,
    pub user: User<Id>,
    pub device: Device<Id>,
    pub public_ip: IpAddr,
}

#[derive(Debug)]
pub enum SessionManagerEventType {
    ClientConnected,
    ClientDisconnected,
    MfaClientConnected,
    MfaClientDisconnected,
}
