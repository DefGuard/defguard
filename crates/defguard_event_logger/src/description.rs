//! Event description generation for activity log.
//!
//! This module provides functions to generate human-readable descriptions for various
//! types of events that occur within the system. These descriptions are used to provide usable
//! context about what happened during each event.
//!
//! Each event type has its own description generator function that takes the event data
//! and returns an optional description string. Some events may not require additional
//! description beyond their event type name, in which case `None` is returned.

use crate::message::{ClientEvent, DefguardEvent, EnrollmentEvent, VpnEvent};

pub fn get_defguard_event_description(event: &DefguardEvent) -> Option<String> {
    match event {
        DefguardEvent::UserLogin => None,
        DefguardEvent::UserLoginFailed { message } => {
            Some(format!("User login failed with: {message}"))
        }
        DefguardEvent::UserMfaLogin { mfa_method } => {
            Some(format!("User logged in using {mfa_method}"))
        }
        DefguardEvent::UserMfaLoginFailed {
            mfa_method,
            message,
        } => Some(format!(
            "User login using {mfa_method} failed with: {message}"
        )),
        DefguardEvent::UserLogout => None,
        DefguardEvent::RecoveryCodeUsed => None,
        DefguardEvent::PasswordChanged => None,
        DefguardEvent::MfaDisabled => todo!(),
        DefguardEvent::MfaTotpEnabled => todo!(),
        DefguardEvent::MfaTotpDisabled => todo!(),
        DefguardEvent::MfaEmailEnabled => todo!(),
        DefguardEvent::MfaEmailDisabled => todo!(),
        DefguardEvent::MfaSecurityKeyAdded { key_id, key_name } => todo!(),
        DefguardEvent::MfaSecurityKeyRemoved { key_id, key_name } => todo!(),
        DefguardEvent::AuthenticationKeyAdded {
            key_id,
            key_name,
            key_type,
        } => todo!(),
        DefguardEvent::AuthenticationKeyRemoved {
            key_id,
            key_name,
            key_type,
        } => todo!(),
        DefguardEvent::AuthenticationKeyRenamed {
            key_id,
            key_name,
            key_type,
        } => todo!(),
        DefguardEvent::ApiTokenAdded {
            token_id,
            token_name,
        } => todo!(),
        DefguardEvent::ApiTokenRemoved {
            token_id,
            token_name,
        } => todo!(),
        DefguardEvent::ApiTokenRenamed {
            token_id,
            token_name,
        } => todo!(),
        DefguardEvent::UserAdded { username } => todo!(),
        DefguardEvent::UserRemoved { username } => todo!(),
        DefguardEvent::UserModified { username } => todo!(),
        DefguardEvent::UserDisabled { username } => todo!(),
        DefguardEvent::UserDeviceAdded {
            device_id,
            device_name,
            owner,
        } => todo!(),
        DefguardEvent::UserDeviceRemoved {
            device_id,
            device_name,
            owner,
        } => todo!(),
        DefguardEvent::UserDeviceModified {
            device_id,
            device_name,
            owner,
        } => todo!(),
        DefguardEvent::NetworkDeviceAdded {
            device_id,
            device_name,
            location_id,
            location,
        } => todo!(),
        DefguardEvent::NetworkDeviceRemoved {
            device_id,
            device_name,
            location_id,
            location,
        } => todo!(),
        DefguardEvent::NetworkDeviceModified {
            device_id,
            device_name,
            location_id,
            location,
        } => todo!(),
        DefguardEvent::VpnLocationAdded {
            location_id,
            location_name,
        } => todo!(),
        DefguardEvent::VpnLocationRemoved {
            location_id,
            location_name,
        } => todo!(),
        DefguardEvent::VpnLocationModified {
            location_id,
            location_name,
        } => todo!(),
        DefguardEvent::OpenIdAppAdded { app_id, app_name } => todo!(),
        DefguardEvent::OpenIdAppRemoved { app_id, app_name } => todo!(),
        DefguardEvent::OpenIdAppModified { app_id, app_name } => todo!(),
        DefguardEvent::OpenIdAppDisabled { app_id, app_name } => todo!(),
        DefguardEvent::OpenIdProviderAdded {
            provider_id,
            provider_name,
        } => todo!(),
        DefguardEvent::OpenIdProviderRemoved {
            provider_id,
            provider_name,
        } => todo!(),
        DefguardEvent::SettingsUpdated => todo!(),
        DefguardEvent::SettingsUpdatedPartial => todo!(),
        DefguardEvent::SettingsDefaultBrandingRestored => todo!(),
        DefguardEvent::ActivityLogStreamCreated {
            stream_id,
            stream_name,
        } => todo!(),
        DefguardEvent::ActivityLogStreamModified {
            stream_id,
            stream_name,
        } => todo!(),
        DefguardEvent::ActivityLogStreamRemoved {
            stream_id,
            stream_name,
        } => todo!(),
    }
}

pub fn get_client_event_description(event: &ClientEvent) -> Option<String> {
    todo!()
}

pub fn get_vpn_event_description(event: &VpnEvent) -> Option<String> {
    todo!()
}

pub fn get_enrollment_event_description(event: &EnrollmentEvent) -> Option<String> {
    todo!()
}
