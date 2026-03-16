use std::net::IpAddr;

use chrono::NaiveDateTime;
use defguard_common::db::{
    models::{Device, User, WireguardNetwork},
    Id,
};

#[derive(Debug)]
pub struct SessionManagerEvent {
    pub context: SessionManagerEventContext,
    pub event: SessionManagerEventType,
}

impl SessionManagerEvent {
    #[must_use]
    pub fn connected_for_session(
        context: SessionManagerEventContext,
        is_mfa_session: bool,
    ) -> Self {
        let event = if is_mfa_session {
            SessionManagerEventType::MfaClientConnected
        } else {
            SessionManagerEventType::ClientConnected
        };

        Self { context, event }
    }

    #[must_use]
    pub fn disconnected_for_session(
        context: SessionManagerEventContext,
        is_mfa_session: bool,
    ) -> Self {
        let event = if is_mfa_session {
            SessionManagerEventType::MfaClientDisconnected
        } else {
            SessionManagerEventType::ClientDisconnected
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
    pub public_ip: IpAddr,
}

#[derive(Debug)]
pub enum SessionManagerEventType {
    ClientConnected,
    ClientDisconnected,
    MfaClientConnected,
    MfaClientDisconnected,
}
