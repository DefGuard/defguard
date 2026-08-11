//! Event description generation for activity log.
//!
//! This module provides functions to generate human-readable descriptions for various
//! types of events that occur within the system. These descriptions are used to provide usable
//! context about what happened during each event.

use defguard_core::events::ApiEventType;

#[must_use]
pub fn get_api_event_description(event: &ApiEventType) -> Option<String> {
    match event {
        ApiEventType::UserLogin
        | ApiEventType::UserLogout
        | ApiEventType::RecoveryCodeUsed
        | ApiEventType::PasswordChanged => None,
        ApiEventType::UserLoginFailed { message } => {
            Some(format!("User login failed with: {message}"))
        }
        ApiEventType::UserMfaLogin { mfa_method } => {
            Some(format!("User logged in using {mfa_method}"))
        }
        ApiEventType::UserMfaLoginFailed {
            mfa_method,
            message,
        } => Some(format!(
            "User login using {mfa_method} failed with: {message}"
        )),
        ApiEventType::RecoveryCodeLoginFailed => {
            Some("User login with recovery code failed".into())
        }
        ApiEventType::MfaDisabled => Some("Disabled own MFA".into()),
        ApiEventType::UserMfaDisabled { user } => Some(format!("Disabled MFA for user {user}")),
        ApiEventType::MfaTotpEnabled => Some("User configured TOTP for MFA".into()),
        ApiEventType::MfaTotpDisabled => Some("User disabled TOTP for MFA".into()),
        ApiEventType::MfaEmailEnabled => Some("User configured email for MFA".into()),
        ApiEventType::MfaEmailDisabled => Some("User disabled email for MFA".into()),
        ApiEventType::PasswordChangedByAdmin { user } => {
            Some(format!("Password for user {user} was changed by an admin"))
        }
        ApiEventType::PasswordReset { user } => Some(format!("Password for user {user} was reset")),
        ApiEventType::MfaSecurityKeyAdded { key } => {
            Some(format!("Added MFA security key {}", key.name))
        }
        ApiEventType::MfaSecurityKeyRemoved { key } => {
            Some(format!("Removed MFA security key {}", key.name))
        }
        ApiEventType::UserAdded { user } => {
            let self_enrollment_enabled = !user.is_enrolled();
            let enrollment_flag_text = if self_enrollment_enabled {
                "enabled"
            } else {
                "disabled"
            };
            Some(format!(
                "Added user {user} with email {} and self-enrollment {enrollment_flag_text}",
                user.email
            ))
        }
        ApiEventType::UserImportBlocked {
            username,
            email,
            user_count,
            limit,
        } => Some(format!(
            "Blocked automatic creation of account for user {username} (email: {email}) \
            because the license user limit has been reached ({user_count}/{limit})"
        )),
        ApiEventType::UserRemoved { user } => Some(format!("Removed user {user}")),
        ApiEventType::UserModified { before, after } => {
            let mut description = format!("Modified user {after}");
            if before.is_active != after.is_active {
                let status_change_text = if after.is_active {
                    "enabled"
                } else {
                    "disabled"
                };
                description = format!("{description}, status changed to {status_change_text}");
            }
            Some(description)
        }
        ApiEventType::UserGroupsModified {
            user,
            before,
            after,
        } => Some(format!(
            "User groups modified! User:{user} Before: {before:?} After {after:?}"
        )),
        ApiEventType::UserEnabled { user } => Some(format!("Enabled user {user}")),
        ApiEventType::UserDisabled { user } => Some(format!("Disabled user {user}")),
        ApiEventType::UserDeviceAdded { owner, device } => {
            Some(format!("Added device {device} for user {owner}"))
        }
        ApiEventType::UserDeviceRemoved { owner, device } => {
            Some(format!("Removed device {device} owned by user {owner}"))
        }
        ApiEventType::UserDeviceModified {
            owner,
            before: _,
            after,
        } => Some(format!("Modified device {after} owned by user {owner}")),
        ApiEventType::NetworkDeviceAdded { device, location } => Some(format!(
            "Added network device {device} to location {location}"
        )),
        ApiEventType::NetworkDeviceRemoved { device, location } => Some(format!(
            "Removed network device {device} from location {location}"
        )),
        ApiEventType::NetworkDeviceModified {
            before: _,
            after,
            location,
        } => Some(format!(
            "Modified network device {after} in location {location}"
        )),
        ApiEventType::ActivityLogStreamCreated { stream } => Some(format!(
            "Created {} activity log stream {}",
            stream.stream_type, stream.name
        )),
        ApiEventType::ActivityLogStreamModified { before: _, after } => Some(format!(
            "Modified {} activity log stream {}",
            after.stream_type, after.name
        )),
        ApiEventType::ActivityLogStreamRemoved { stream } => Some(format!(
            "Removed {} activity log stream {}",
            stream.stream_type, stream.name
        )),
        ApiEventType::VpnLocationAdded { location } => {
            Some(format!("Added VPN location {location}"))
        }
        ApiEventType::VpnLocationRemoved { location } => {
            Some(format!("Removed VPN location {location}"))
        }
        ApiEventType::VpnLocationModified { before: _, after } => {
            Some(format!("VPN location {after} was modified"))
        }
        ApiEventType::ApiTokenAdded { owner, token } => {
            Some(format!("Added API token {} for user {owner}", token.name))
        }
        ApiEventType::ApiTokenRemoved { owner, token } => Some(format!(
            "Removed API token {} owned by user {owner}",
            token.name
        )),
        ApiEventType::ApiTokenRenamed {
            owner,
            token: _,
            old_name,
            new_name,
        } => Some(format!(
            "API token owned by user {owner} was renamed from {old_name} to {new_name}",
        )),
        ApiEventType::OpenIdAppAdded { app } => {
            Some(format!("Added OpenID application {}", app.name))
        }
        ApiEventType::OpenIdAppRemoved { app } => {
            Some(format!("Removed OpenID application {}", app.name))
        }
        ApiEventType::OpenIdAppModified { before: _, after } => {
            Some(format!("Modified OpenID application {}", after.name))
        }
        ApiEventType::OpenIdAppStateChanged { app, enabled } => {
            let state = if *enabled { "Enabled" } else { "Disabled" };
            Some(format!("{} OpenID application {}", state, app.name))
        }
        ApiEventType::OpenIdProviderModified { provider } => {
            Some(format!("Modified OpenID provider {}", provider.name))
        }
        ApiEventType::OpenIdProviderRemoved { provider } => {
            Some(format!("Removed OpenID provider {}", provider.name))
        }
        ApiEventType::SettingsUpdated {
            before: _,
            after: _,
        } => None,
        ApiEventType::SettingsUpdatedPartial {
            before: _,
            after: _,
        } => None,
        ApiEventType::SettingsDefaultBrandingRestored => {
            Some("Restored default branding settings".into())
        }
        ApiEventType::EnterpriseSettingsUpdated {
            before: _,
            after: _,
        } => None,
        ApiEventType::GroupsBulkAssigned { users, groups } => Some(format!(
            "Assigned {} users to {} groups",
            users.len(),
            groups.len()
        )),
        ApiEventType::GroupAdded { group } => Some(format!("Added group {}", group.name)),
        ApiEventType::GroupModified { before: _, after } => {
            Some(format!("Modified group {}", after.name))
        }
        ApiEventType::GroupRemoved { group } => Some(format!("Removed group {}", group.name)),
        ApiEventType::GroupMemberAdded { group, user } => {
            Some(format!("Added user {user} to group {}", group.name))
        }
        ApiEventType::GroupMemberRemoved { group, user } => {
            Some(format!("Removed user {user} from group {}", group.name))
        }
        ApiEventType::GroupMembersModified {
            group,
            added,
            removed,
        } => Some(format!(
            "Added: {}, Removed: {}, for group {}",
            added
                .iter()
                .map(|user| user.username.clone())
                .collect::<Vec<_>>()
                .join(", "),
            removed
                .iter()
                .map(|user| user.username.clone())
                .collect::<Vec<_>>()
                .join(", "),
            group.name
        )),
        ApiEventType::WebHookAdded { webhook } => {
            Some(format!("Added webhook with URL {}", webhook.url))
        }
        ApiEventType::WebHookModified { before: _, after } => {
            Some(format!("Modified webhook with URL {}", after.url))
        }
        ApiEventType::WebHookRemoved { webhook } => {
            Some(format!("Removed webhook with URL {}", webhook.url))
        }
        ApiEventType::WebHookStateChanged { webhook, enabled } => {
            let state = if *enabled { "Enabled" } else { "Disabled" };
            Some(format!("{} webhook with URL {}", state, webhook.url))
        }
        ApiEventType::AuthenticationKeyAdded { key } => Some(format!(
            "Added {} authentication key {}",
            key.key_type,
            key.name.clone().unwrap_or_default()
        )),
        ApiEventType::AuthenticationKeyRemoved { key } => Some(format!(
            "Removed {} authentication key {}",
            key.key_type,
            key.name.clone().unwrap_or_default()
        )),
        ApiEventType::AuthenticationKeyRenamed {
            key,
            old_name,
            new_name,
        } => Some(format!(
            "Renamed {} authentication key from {} to {}",
            key.key_type,
            old_name.clone().unwrap_or_default(),
            new_name.clone().unwrap_or_default()
        )),
        ApiEventType::ClientConfigurationTokenAdded { user } => {
            Some(format!("Added client configuration token for user {user}"))
        }
        ApiEventType::UserSnatBindingAdded { user, binding, .. } => Some(format!(
            "Devices owned by user {user} bound to public IP {}",
            binding.public_ip
        )),
        ApiEventType::UserSnatBindingRemoved { user, binding, .. } => Some(format!(
            "Removed public IP {} binding for user {user}",
            binding.public_ip
        )),
        ApiEventType::UserSnatBindingModified {
            user,
            before,
            after,
            ..
        } => Some(format!(
            "Public IP bound to devices owned by user {user} changed from {} to {}",
            before.public_ip, after.public_ip
        )),
        ApiEventType::ProxyModified { before: _, after } => Some(format!("Modified proxy {after}")),
        ApiEventType::ProxyDeleted { proxy } => Some(format!("Deleted proxy {proxy}")),
        ApiEventType::GatewayModified { before: _, after } => {
            Some(format!("Modified gateway {after}"))
        }
        ApiEventType::GatewayDeleted { gateway } => Some(format!("Deleted gateway {gateway}")),
        ApiEventType::DevicePostureCreated { snapshot } => Some(format!(
            "Created device posture check {}",
            snapshot.device_posture.name
        )),
        ApiEventType::DevicePostureUpdated { after, .. } => Some(format!(
            "Updated device posture check {}",
            after.device_posture.name
        )),
        ApiEventType::DevicePostureDeleted { snapshot } => Some(format!(
            "Deleted device posture check {}",
            snapshot.device_posture.name
        )),
        ApiEventType::DevicePostureDuplicated { duplicate, .. } => Some(format!(
            "Duplicated device posture check as {}",
            duplicate.device_posture.name
        )),
        ApiEventType::DevicePostureLocationsAssigned {
            device_posture,
            location_ids,
        } => Some(format!(
            "Assigned {} location(s) to device posture check {}",
            location_ids.len(),
            device_posture.id
        )),
        ApiEventType::LocationPosturesAssigned {
            location,
            posture_ids,
        } => Some(format!(
            "Assigned {} posture check(s) to location {}",
            posture_ids.len(),
            location.id
        )),
        ApiEventType::MfaFlowCreated { snapshot } => {
            Some(format!("Created MFA flow '{}'", snapshot.flow.title))
        }
        ApiEventType::MfaFlowUpdated { after, .. } => {
            Some(format!("Updated MFA flow '{}'", after.flow.title))
        }
        ApiEventType::MfaFlowDeleted { snapshot } => {
            Some(format!("Deleted MFA flow '{}'", snapshot.flow.title))
        }
        ApiEventType::LocationMfaFlowsAssigned {
            location_name,
            assignment_count,
            ..
        } => Some(format!(
            "Assigned {assignment_count} MFA flow(s) to location '{location_name}'"
        )),
        ApiEventType::EnrollmentTokenAdded { user } => {
            Some(format!("Added enrollment token for user {user}"))
        }
    }
}

#[must_use]
pub fn get_enrollment_event_description(
    event: &defguard_core::events::EnrollmentEvent,
) -> Option<String> {
    match event {
        defguard_core::events::EnrollmentEvent::EnrollmentStarted => {
            Some("User started enrollment process".into())
        }
        defguard_core::events::EnrollmentEvent::EnrollmentDeviceAdded { device } => {
            Some(format!("Added device {} during enrollment", device.name))
        }
        defguard_core::events::EnrollmentEvent::EnrollmentCompleted => {
            Some("User completed enrollment process".into())
        }
    }
}
