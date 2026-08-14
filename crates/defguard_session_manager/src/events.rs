use std::net::IpAddr;

use chrono::NaiveDateTime;
use defguard_common::db::{
    Id,
    models::{Device, User, WireguardNetwork, vpn_client_session::VpnClientMfaMethod},
};
use strum::EnumCount;

#[derive(Debug)]
pub struct SessionManagerEvent {
    pub context: SessionManagerEventContext,
    pub event: SessionManagerEventType,
}

impl SessionManagerEvent {
    #[must_use]
    pub fn connected_for_session(context: SessionManagerEventContext) -> Self {
        let event = if context.mfa_methods.is_empty() {
            SessionManagerEventType::ClientConnected
        } else {
            SessionManagerEventType::MfaClientConnected
        };

        Self { context, event }
    }

    #[must_use]
    pub fn disconnected_for_session(context: SessionManagerEventContext) -> Self {
        let event = if context.mfa_methods.is_empty() {
            SessionManagerEventType::ClientDisconnected
        } else {
            SessionManagerEventType::MfaClientDisconnected
        };

        Self { context, event }
    }
}

#[derive(Debug)]
pub struct SessionManagerEventContext {
    pub timestamp: NaiveDateTime,
    pub location: WireguardNetwork<Id>,
    pub user: User<Id>,
    pub device: Device<Id>,
    pub public_ip: Option<IpAddr>,
    pub mfa_methods: Vec<VpnClientMfaMethod>,
}

#[derive(Debug, EnumCount)]
pub enum SessionManagerEventType {
    ClientConnected,
    ClientDisconnected,
    MfaClientConnected,
    MfaClientDisconnected,
}
