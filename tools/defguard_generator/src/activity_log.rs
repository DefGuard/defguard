use anyhow::Result;
use chrono::{Duration, Utc};
use defguard_common::db::{
    Id, NoId,
    models::{
        Device, DeviceType, MFAMethod, Settings, User, WebAuthn, WireguardNetwork, group::Group,
    },
};
use defguard_core::{
    db::models::activity_log::{
        ActivityLogEvent, ActivityLogModule, EventType,
        metadata::{
            DeviceMetadata, EnrollmentDeviceAddedMetadata, EnrollmentTokenMetadata,
            GroupAssignedMetadata, GroupsBulkAssignedMetadata, LoginFailedMetadata,
            MfaLoginFailedMetadata, MfaLoginMetadata, MfaSecurityKeyMetadata,
            NetworkDeviceMetadata, PasswordChangedByAdminMetadata, PasswordResetMetadata,
            UserMetadata, UserMfaDisabledMetadata, VpnClientMetadata, VpnClientMfaMetadata,
        },
    },
    events::{ApiEventType, ClientMFAMethod, EnrollmentEvent as CoreEnrollmentEvent},
};
use defguard_event_logger::description::{
    get_api_event_description,
    get_enrollment_event_description as get_core_enrollment_event_description,
};
use rand::{Rng, rngs::ThreadRng, seq::SliceRandom};

#[allow(dead_code)]
#[allow(clippy::large_enum_variant)]
enum DefguardEvent {
    UserLogin,
    UserLoginFailed {
        message: String,
    },
    UserLogout,
    UserMfaLogin {
        mfa_method: MFAMethod,
    },
    UserMfaLoginFailed {
        mfa_method: MFAMethod,
        message: String,
    },
    RecoveryCodeLoginFailed,
    RecoveryCodeUsed,
    PasswordChangedByAdmin {
        user: User<Id>,
    },
    PasswordChanged,
    PasswordReset {
        user: User<Id>,
    },
    MfaDisabled,
    UserMfaDisabled {
        user: User<Id>,
    },
    MfaTotpDisabled,
    MfaTotpEnabled,
    MfaEmailDisabled,
    MfaEmailEnabled,
    MfaSecurityKeyAdded {
        key: WebAuthn<Id>,
    },
    MfaSecurityKeyRemoved {
        key: WebAuthn<Id>,
    },
    UserAdded {
        user: User<Id>,
    },
    UserRemoved {
        user: User<Id>,
    },
    UserModified {
        before: User<Id>,
        after: User<Id>,
    },
    UserGroupsModified {
        user: User<Id>,
        before: Vec<String>,
        after: Vec<String>,
    },
    UserDeviceAdded {
        owner: User<Id>,
        device: Device<Id>,
    },
    UserDeviceRemoved {
        owner: User<Id>,
        device: Device<Id>,
    },
    NetworkDeviceAdded {
        device: Device<Id>,
        location: WireguardNetwork<Id>,
    },
    NetworkDeviceRemoved {
        device: Device<Id>,
        location: WireguardNetwork<Id>,
    },
    GroupMemberAdded {
        group: Group<Id>,
        user: User<Id>,
    },
    GroupMemberRemoved {
        group: Group<Id>,
        user: User<Id>,
    },
    GroupsBulkAssigned {
        users: Vec<User<Id>>,
        groups: Vec<Group<Id>>,
    },
}

impl DefguardEvent {
    fn to_api_event_type(&self) -> Option<ApiEventType> {
        match self {
            DefguardEvent::UserLogin => Some(ApiEventType::UserLogin),
            DefguardEvent::UserLoginFailed { message } => Some(ApiEventType::UserLoginFailed {
                message: message.clone(),
            }),
            DefguardEvent::UserLogout => Some(ApiEventType::UserLogout),
            DefguardEvent::UserMfaLogin { mfa_method } => Some(ApiEventType::UserMfaLogin {
                mfa_method: *mfa_method,
            }),
            DefguardEvent::UserMfaLoginFailed {
                mfa_method,
                message,
            } => Some(ApiEventType::UserMfaLoginFailed {
                mfa_method: *mfa_method,
                message: message.clone(),
            }),
            DefguardEvent::RecoveryCodeLoginFailed => Some(ApiEventType::RecoveryCodeLoginFailed),
            DefguardEvent::RecoveryCodeUsed => Some(ApiEventType::RecoveryCodeUsed),
            DefguardEvent::PasswordChangedByAdmin { user } => {
                Some(ApiEventType::PasswordChangedByAdmin { user: user.clone() })
            }
            DefguardEvent::PasswordChanged => Some(ApiEventType::PasswordChanged),
            DefguardEvent::PasswordReset { user } => {
                Some(ApiEventType::PasswordReset { user: user.clone() })
            }
            DefguardEvent::MfaDisabled => Some(ApiEventType::MfaDisabled),
            DefguardEvent::UserMfaDisabled { user } => {
                Some(ApiEventType::UserMfaDisabled { user: user.clone() })
            }
            DefguardEvent::MfaTotpDisabled => Some(ApiEventType::MfaTotpDisabled),
            DefguardEvent::MfaTotpEnabled => Some(ApiEventType::MfaTotpEnabled),
            DefguardEvent::MfaEmailDisabled => Some(ApiEventType::MfaEmailDisabled),
            DefguardEvent::MfaEmailEnabled => Some(ApiEventType::MfaEmailEnabled),
            DefguardEvent::MfaSecurityKeyAdded { key } => {
                Some(ApiEventType::MfaSecurityKeyAdded { key: key.clone() })
            }
            DefguardEvent::MfaSecurityKeyRemoved { key } => {
                Some(ApiEventType::MfaSecurityKeyRemoved { key: key.clone() })
            }
            DefguardEvent::UserAdded { user } => {
                Some(ApiEventType::UserAdded { user: user.clone() })
            }
            DefguardEvent::UserRemoved { user } => {
                Some(ApiEventType::UserRemoved { user: user.clone() })
            }
            DefguardEvent::UserModified { before, after } => Some(ApiEventType::UserModified {
                before: before.clone(),
                after: after.clone(),
            }),
            DefguardEvent::UserGroupsModified {
                user,
                before,
                after,
            } => Some(ApiEventType::UserGroupsModified {
                user: user.clone(),
                before: before.clone(),
                after: after.clone(),
            }),
            DefguardEvent::UserDeviceAdded { owner, device } => {
                Some(ApiEventType::UserDeviceAdded {
                    owner: owner.clone(),
                    device: device.clone(),
                })
            }
            DefguardEvent::UserDeviceRemoved { owner, device } => {
                Some(ApiEventType::UserDeviceRemoved {
                    owner: owner.clone(),
                    device: device.clone(),
                })
            }
            DefguardEvent::NetworkDeviceAdded { device, location } => {
                Some(ApiEventType::NetworkDeviceAdded {
                    device: device.clone(),
                    location: location.clone(),
                })
            }
            DefguardEvent::NetworkDeviceRemoved { device, location } => {
                Some(ApiEventType::NetworkDeviceRemoved {
                    device: device.clone(),
                    location: location.clone(),
                })
            }
            DefguardEvent::GroupMemberAdded { group, user } => {
                Some(ApiEventType::GroupMemberAdded {
                    group: group.clone(),
                    user: user.clone(),
                })
            }
            DefguardEvent::GroupMemberRemoved { group, user } => {
                Some(ApiEventType::GroupMemberRemoved {
                    group: group.clone(),
                    user: user.clone(),
                })
            }
            DefguardEvent::GroupsBulkAssigned { users, groups } => {
                Some(ApiEventType::GroupsBulkAssigned {
                    users: users.clone(),
                    groups: groups.clone(),
                })
            }
        }
    }
}

#[allow(dead_code)]
enum VpnEvent {
    ClientMfaSuccess {
        location: WireguardNetwork<Id>,
        device: Device<Id>,
        method: ClientMFAMethod,
    },
    ClientMfaFailed {
        location: WireguardNetwork<Id>,
        device: Device<Id>,
        method: ClientMFAMethod,
        message: String,
    },
    ConnectedToLocation {
        location: WireguardNetwork<Id>,
        device: Device<Id>,
    },
    DisconnectedFromLocation {
        location: WireguardNetwork<Id>,
        device: Device<Id>,
    },
    MfaConnectedToLocation {
        location: WireguardNetwork<Id>,
        device: Device<Id>,
    },
    MfaDisconnectedFromLocation {
        location: WireguardNetwork<Id>,
        device: Device<Id>,
    },
}

#[allow(dead_code)]
#[allow(clippy::large_enum_variant)]
enum EnrollmentEvent {
    EnrollmentStarted,
    EnrollmentDeviceAdded { device: Device<Id> },
    EnrollmentCompleted,
    PasswordResetRequested,
    PasswordResetStarted,
    PasswordResetCompleted,
    TokenAdded { user: User<Id> },
}

fn get_defguard_event_description(event: &DefguardEvent) -> Option<String> {
    event
        .to_api_event_type()
        .as_ref()
        .and_then(get_api_event_description)
}

fn get_vpn_event_description(event: &VpnEvent) -> Option<String> {
    match event {
        VpnEvent::ConnectedToLocation { location, device } => {
            Some(format!("Device {device} connected to location {location}"))
        }
        VpnEvent::DisconnectedFromLocation { location, device } => Some(format!(
            "Device {device} disconnected from location {location}"
        )),
        VpnEvent::MfaConnectedToLocation { location, device } => Some(format!(
            "Device {device} connected to MFA location {location}"
        )),
        VpnEvent::MfaDisconnectedFromLocation { location, device } => Some(format!(
            "Device {device} disconnected from MFA location {location}"
        )),
        VpnEvent::ClientMfaSuccess {
            location,
            device,
            method,
        } => Some(format!(
            "Device {device} completed MFA authorization for location {location} using {method}"
        )),
        VpnEvent::ClientMfaFailed {
            location,
            device,
            method,
            message,
        } => Some(format!(
            "Device {device} failed to connect to MFA location {location} using {method} with: {message}"
        )),
    }
}

fn get_enrollment_event_description(event: &EnrollmentEvent) -> Option<String> {
    match event {
        EnrollmentEvent::TokenAdded { user } => {
            Some(format!("Added enrollment token for user {user}"))
        }
        EnrollmentEvent::PasswordResetRequested
        | EnrollmentEvent::PasswordResetStarted
        | EnrollmentEvent::PasswordResetCompleted => None,
        EnrollmentEvent::EnrollmentStarted => {
            get_core_enrollment_event_description(&CoreEnrollmentEvent::EnrollmentStarted)
        }
        EnrollmentEvent::EnrollmentDeviceAdded { device } => {
            get_core_enrollment_event_description(&CoreEnrollmentEvent::EnrollmentDeviceAdded {
                device: device.clone(),
            })
        }
        EnrollmentEvent::EnrollmentCompleted => {
            get_core_enrollment_event_description(&CoreEnrollmentEvent::EnrollmentCompleted)
        }
    }
}
use sqlx::PgPool;
use tracing::info;

use crate::{user_devices::prepare_user_devices, users::prepare_users};

pub const DEFAULT_NUM_EVENTS: usize = 20;
pub const DEFAULT_TIME_SPAN_MINUTES: i64 = 1;

const USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
     Chrome/126.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) \
     Version/17.5 Safari/605.1.15",
    "Mozilla/5.0 (X11; Linux x86_64; rv:127.0) Gecko/20100101 Firefox/127.0",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like \
     Gecko) Version/17.5 Mobile/15E148 Safari/604.1",
    "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) \
     Chrome/126.0.0.0 Mobile Safari/537.36",
];

const SECURITY_KEY_NAMES: &[&str] = &[
    "YubiKey 5 NFC",
    "YubiKey 5C",
    "Titan Security Key",
    "Passkey",
    "TouchID",
];

const NETWORK_DEVICE_NAMES: &[&str] = &[
    "Office Router",
    "Firewall",
    "NAS",
    "Print Server",
    "Backup Server",
];

#[derive(Debug)]
pub struct ActivityLogGeneratorConfig {
    pub num_events: usize,
    pub time_span_minutes: i64,
    pub num_users: usize,
}

#[derive(Clone, Copy)]
enum EventKind {
    // authentication
    Login,
    Logout,
    MfaLogin,
    LoginFailed,
    MfaLoginFailed,
    RecoveryCodeUsed,
    RecoveryCodeLoginFailed,
    PasswordChanged,
    // MFA management
    MfaDisabled,
    UserMfaDisabled,
    MfaTotpEnabled,
    MfaTotpDisabled,
    MfaEmailEnabled,
    MfaEmailDisabled,
    MfaSecurityKeyAdded,
    MfaSecurityKeyRemoved,
    // user management
    UserAdded,
    UserRemoved,
    PasswordChangedByAdmin,
    PasswordReset,
    // device management
    DeviceAdded,
    DeviceRemoved,
    NetworkDeviceAdded,
    NetworkDeviceRemoved,
    // group management
    GroupMemberAdded,
    GroupMemberRemoved,
    GroupsBulkAssigned,
    // enrollment
    EnrollmentStarted,
    EnrollmentDeviceAdded,
    EnrollmentCompleted,
    EnrollmentTokenAdded,
    PasswordResetRequested,
    PasswordResetStarted,
    PasswordResetCompleted,
    // VPN
    VpnConnected,
    VpnDisconnected,
    VpnMfaConnected,
    VpnMfaDisconnected,
    VpnMfaSuccess,
}

struct BuildContext<'a> {
    user: &'a User<Id>,
    device: &'a Device<Id>,
    locations: &'a [WireguardNetwork<Id>],
    groups: &'a [Group<Id>],
    users: &'a [User<Id>],
}

struct GeneratedEvent {
    module: ActivityLogModule,
    event_type: EventType,
    description: Option<String>,
    metadata: Option<serde_json::Value>,
    location: Option<String>,
    device: String,
}

pub async fn generate_activity_log(
    pool: &PgPool,
    config: ActivityLogGeneratorConfig,
) -> Result<()> {
    info!("Running activity log generator with config: {config:#?}");

    let mut rng = rand::thread_rng();

    let mut users = prepare_users(pool, &mut rng, config.num_users.max(1)).await?;

    let default_admin_id = Settings::get(pool)
        .await?
        .and_then(|settings| settings.default_admin_id);
    users.retain(|user| Some(user.id) != default_admin_id && user.username != "admin");

    if users.is_empty() {
        info!("No non-admin users available, skipping activity log generation");
        return Ok(());
    }

    users.shuffle(&mut rng);

    let mut user_devices: Vec<(User<Id>, Device<Id>)> = Vec::with_capacity(users.len());
    for user in &users {
        let device = prepare_user_devices(pool, &mut rng, user, 1)
            .await?
            .into_iter()
            .next()
            .expect("prepare_user_devices always returns at least one device");
        user_devices.push((user.clone(), device));
    }

    let locations = WireguardNetwork::all(pool).await?;
    let locations_available = !locations.is_empty();
    if !locations_available {
        info!("No VPN locations found, skipping VPN and network device events");
    }

    let groups = Group::all(pool).await?;
    let groups_available = !groups.is_empty();
    if !groups_available {
        info!("No groups found, skipping group membership events");
    }

    let kind_pool = build_kind_pool(locations_available, groups_available);

    let now = Utc::now().naive_utc();
    let span_seconds = Duration::minutes(config.time_span_minutes.max(1))
        .num_seconds()
        .max(1);

    info!("Generating {} activity log events", config.num_events);

    for _ in 0..config.num_events {
        let (user, device) = user_devices
            .choose(&mut rng)
            .expect("user_devices is non-empty");
        let kind = *kind_pool.choose(&mut rng).expect("kind_pool is non-empty");

        let timestamp = now - Duration::seconds(rng.gen_range(0..span_seconds));

        let ctx = BuildContext {
            user,
            device,
            locations: &locations,
            groups: &groups,
            users: &users,
        };
        let generated = build_event(&mut rng, &ctx, kind);

        let event = ActivityLogEvent {
            id: NoId,
            timestamp,
            user_id: Some(user.id),
            username: user.username.clone(),
            location: generated.location,
            ip: None,
            event: generated.event_type,
            module: generated.module,
            device: generated.device,
            description: generated.description,
            metadata: generated.metadata,
        };

        event.save(pool).await?;
    }

    info!("Finished generating activity log events");

    Ok(())
}

fn build_kind_pool(locations_available: bool, groups_available: bool) -> Vec<EventKind> {
    use EventKind::*;

    let mut weighted: Vec<(EventKind, u8)> = vec![
        (Login, 8),
        (Logout, 6),
        (MfaLogin, 6),
        (LoginFailed, 3),
        (MfaLoginFailed, 2),
        (RecoveryCodeUsed, 1),
        (RecoveryCodeLoginFailed, 1),
        (PasswordChanged, 2),
        (MfaDisabled, 1),
        (UserMfaDisabled, 1),
        (MfaTotpEnabled, 1),
        (MfaTotpDisabled, 1),
        (MfaEmailEnabled, 1),
        (MfaEmailDisabled, 1),
        (MfaSecurityKeyAdded, 1),
        (MfaSecurityKeyRemoved, 1),
        (UserAdded, 2),
        (UserRemoved, 1),
        (PasswordChangedByAdmin, 1),
        (PasswordReset, 1),
        (DeviceAdded, 3),
        (DeviceRemoved, 2),
        (EnrollmentStarted, 2),
        (EnrollmentDeviceAdded, 2),
        (EnrollmentCompleted, 2),
        (EnrollmentTokenAdded, 1),
        (PasswordResetRequested, 1),
        (PasswordResetStarted, 1),
        (PasswordResetCompleted, 1),
    ];

    if locations_available {
        weighted.extend([
            (VpnConnected, 8),
            (VpnDisconnected, 8),
            (VpnMfaConnected, 4),
            (VpnMfaDisconnected, 4),
            (VpnMfaSuccess, 3),
            (NetworkDeviceAdded, 1),
            (NetworkDeviceRemoved, 1),
        ]);
    }

    if groups_available {
        weighted.extend([
            (GroupMemberAdded, 2),
            (GroupMemberRemoved, 1),
            (GroupsBulkAssigned, 1),
        ]);
    }

    weighted
        .into_iter()
        .flat_map(|(kind, weight)| std::iter::repeat_n(kind, weight as usize))
        .collect()
}

fn build_event(rng: &mut ThreadRng, ctx: &BuildContext, kind: EventKind) -> GeneratedEvent {
    let user_agent = random_user_agent(rng).to_string();

    let defguard = |event_type: EventType,
                    metadata: Option<serde_json::Value>,
                    description: Option<String>|
     -> GeneratedEvent {
        GeneratedEvent {
            module: ActivityLogModule::Defguard,
            event_type,
            description,
            metadata,
            location: None,
            device: user_agent.clone(),
        }
    };

    let enrollment = |event_type: EventType,
                      metadata: Option<serde_json::Value>,
                      description: Option<String>|
     -> GeneratedEvent {
        GeneratedEvent {
            module: ActivityLogModule::Enrollment,
            event_type,
            description,
            metadata,
            location: None,
            device: user_agent.clone(),
        }
    };

    match kind {
        EventKind::Login => defguard(
            EventType::UserLogin,
            None,
            get_defguard_event_description(&DefguardEvent::UserLogin),
        ),
        EventKind::Logout => defguard(
            EventType::UserLogout,
            None,
            get_defguard_event_description(&DefguardEvent::UserLogout),
        ),
        EventKind::MfaLogin => {
            let mfa_method = random_mfa_method(rng);
            defguard(
                EventType::UserMfaLogin,
                serde_json::to_value(MfaLoginMetadata { mfa_method }).ok(),
                get_defguard_event_description(&DefguardEvent::UserMfaLogin { mfa_method }),
            )
        }
        EventKind::LoginFailed => {
            let message = format!(
                "Authentication for {} failed: invalid password",
                ctx.user.username
            );
            defguard(
                EventType::UserLoginFailed,
                serde_json::to_value(LoginFailedMetadata {
                    message: message.clone(),
                })
                .ok(),
                get_defguard_event_description(&DefguardEvent::UserLoginFailed { message }),
            )
        }
        EventKind::MfaLoginFailed => {
            let (mfa_method, message) = if rng.r#gen::<bool>() {
                (
                    MFAMethod::OneTimePassword,
                    "TOTP code verification failed".to_string(),
                )
            } else {
                (
                    MFAMethod::Email,
                    "Email code verification failed".to_string(),
                )
            };
            defguard(
                EventType::UserMfaLoginFailed,
                serde_json::to_value(MfaLoginFailedMetadata {
                    mfa_method,
                    message: message.clone(),
                })
                .ok(),
                get_defguard_event_description(&DefguardEvent::UserMfaLoginFailed {
                    mfa_method,
                    message,
                }),
            )
        }
        EventKind::RecoveryCodeUsed => defguard(
            EventType::RecoveryCodeUsed,
            None,
            get_defguard_event_description(&DefguardEvent::RecoveryCodeUsed),
        ),
        EventKind::RecoveryCodeLoginFailed => defguard(
            EventType::UserMfaLoginFailed,
            serde_json::to_value(LoginFailedMetadata {
                message: "Recovery code verification failed".to_string(),
            })
            .ok(),
            get_defguard_event_description(&DefguardEvent::RecoveryCodeLoginFailed),
        ),
        EventKind::PasswordChanged => defguard(
            EventType::PasswordChanged,
            None,
            get_defguard_event_description(&DefguardEvent::PasswordChanged),
        ),
        EventKind::MfaDisabled => defguard(
            EventType::MfaDisabled,
            None,
            get_defguard_event_description(&DefguardEvent::MfaDisabled),
        ),
        EventKind::UserMfaDisabled => defguard(
            EventType::UserMfaDisabled,
            serde_json::to_value(UserMfaDisabledMetadata {
                user: ctx.user.clone().into(),
            })
            .ok(),
            get_defguard_event_description(&DefguardEvent::UserMfaDisabled {
                user: ctx.user.clone(),
            }),
        ),
        EventKind::MfaTotpEnabled => defguard(
            EventType::MfaTotpEnabled,
            None,
            get_defguard_event_description(&DefguardEvent::MfaTotpEnabled),
        ),
        EventKind::MfaTotpDisabled => defguard(
            EventType::MfaTotpDisabled,
            None,
            get_defguard_event_description(&DefguardEvent::MfaTotpDisabled),
        ),
        EventKind::MfaEmailEnabled => defguard(
            EventType::MfaEmailEnabled,
            None,
            get_defguard_event_description(&DefguardEvent::MfaEmailEnabled),
        ),
        EventKind::MfaEmailDisabled => defguard(
            EventType::MfaEmailDisabled,
            None,
            get_defguard_event_description(&DefguardEvent::MfaEmailDisabled),
        ),
        EventKind::MfaSecurityKeyAdded => {
            let key = fabricate_security_key(rng, ctx.user.id);
            defguard(
                EventType::MfaSecurityKeyAdded,
                serde_json::to_value(MfaSecurityKeyMetadata {
                    key: key.clone().into(),
                })
                .ok(),
                get_defguard_event_description(&DefguardEvent::MfaSecurityKeyAdded { key }),
            )
        }
        EventKind::MfaSecurityKeyRemoved => {
            let key = fabricate_security_key(rng, ctx.user.id);
            defguard(
                EventType::MfaSecurityKeyRemoved,
                serde_json::to_value(MfaSecurityKeyMetadata {
                    key: key.clone().into(),
                })
                .ok(),
                get_defguard_event_description(&DefguardEvent::MfaSecurityKeyRemoved { key }),
            )
        }
        EventKind::UserAdded => defguard(
            EventType::UserAdded,
            serde_json::to_value(UserMetadata {
                user: ctx.user.clone().into(),
            })
            .ok(),
            get_defguard_event_description(&DefguardEvent::UserAdded {
                user: ctx.user.clone(),
            }),
        ),
        EventKind::UserRemoved => defguard(
            EventType::UserRemoved,
            serde_json::to_value(UserMetadata {
                user: ctx.user.clone().into(),
            })
            .ok(),
            get_defguard_event_description(&DefguardEvent::UserRemoved {
                user: ctx.user.clone(),
            }),
        ),
        EventKind::PasswordChangedByAdmin => defguard(
            EventType::PasswordChangedByAdmin,
            serde_json::to_value(PasswordChangedByAdminMetadata {
                user: ctx.user.clone().into(),
            })
            .ok(),
            get_defguard_event_description(&DefguardEvent::PasswordChangedByAdmin {
                user: ctx.user.clone(),
            }),
        ),
        EventKind::PasswordReset => defguard(
            EventType::PasswordReset,
            serde_json::to_value(PasswordResetMetadata {
                user: ctx.user.clone().into(),
            })
            .ok(),
            get_defguard_event_description(&DefguardEvent::PasswordReset {
                user: ctx.user.clone(),
            }),
        ),
        EventKind::DeviceAdded => defguard(
            EventType::DeviceAdded,
            serde_json::to_value(DeviceMetadata {
                owner: ctx.user.clone().into(),
                device: ctx.device.clone(),
            })
            .ok(),
            get_defguard_event_description(&DefguardEvent::UserDeviceAdded {
                owner: ctx.user.clone(),
                device: ctx.device.clone(),
            }),
        ),
        EventKind::DeviceRemoved => defguard(
            EventType::DeviceRemoved,
            serde_json::to_value(DeviceMetadata {
                owner: ctx.user.clone().into(),
                device: ctx.device.clone(),
            })
            .ok(),
            get_defguard_event_description(&DefguardEvent::UserDeviceRemoved {
                owner: ctx.user.clone(),
                device: ctx.device.clone(),
            }),
        ),
        EventKind::NetworkDeviceAdded => {
            build_network_device_event(rng, ctx, &user_agent, EventType::NetworkDeviceAdded)
        }
        EventKind::NetworkDeviceRemoved => {
            build_network_device_event(rng, ctx, &user_agent, EventType::NetworkDeviceRemoved)
        }
        EventKind::GroupMemberAdded => {
            let group = ctx.groups.choose(rng).expect("groups is non-empty").clone();
            defguard(
                EventType::GroupMemberAdded,
                serde_json::to_value(GroupAssignedMetadata {
                    group: group.clone(),
                    user: ctx.user.clone().into(),
                })
                .ok(),
                get_defguard_event_description(&DefguardEvent::GroupMemberAdded {
                    group,
                    user: ctx.user.clone(),
                }),
            )
        }
        EventKind::GroupMemberRemoved => {
            let group = ctx.groups.choose(rng).expect("groups is non-empty").clone();
            defguard(
                EventType::GroupMemberRemoved,
                serde_json::to_value(GroupAssignedMetadata {
                    group: group.clone(),
                    user: ctx.user.clone().into(),
                })
                .ok(),
                get_defguard_event_description(&DefguardEvent::GroupMemberRemoved {
                    group,
                    user: ctx.user.clone(),
                }),
            )
        }
        EventKind::GroupsBulkAssigned => {
            let group_count = rng.gen_range(1..=ctx.groups.len().min(2));
            let groups: Vec<Group<Id>> = ctx
                .groups
                .choose_multiple(rng, group_count)
                .cloned()
                .collect();
            let user_count = rng.gen_range(1..=ctx.users.len().min(5));
            let users: Vec<User<Id>> = ctx
                .users
                .choose_multiple(rng, user_count)
                .cloned()
                .collect();
            defguard(
                EventType::GroupsBulkAssigned,
                serde_json::to_value(GroupsBulkAssignedMetadata {
                    users: users.iter().cloned().map(Into::into).collect(),
                    groups: groups.clone(),
                })
                .ok(),
                get_defguard_event_description(&DefguardEvent::GroupsBulkAssigned {
                    users,
                    groups,
                }),
            )
        }
        EventKind::EnrollmentStarted => enrollment(
            EventType::EnrollmentStarted,
            None,
            get_enrollment_event_description(&EnrollmentEvent::EnrollmentStarted),
        ),
        EventKind::EnrollmentDeviceAdded => {
            let device = ctx.device.clone();
            enrollment(
                EventType::EnrollmentDeviceAdded,
                serde_json::to_value(EnrollmentDeviceAddedMetadata {
                    device: device.clone(),
                })
                .ok(),
                get_enrollment_event_description(&EnrollmentEvent::EnrollmentDeviceAdded {
                    device,
                }),
            )
        }
        EventKind::EnrollmentCompleted => enrollment(
            EventType::EnrollmentCompleted,
            None,
            get_enrollment_event_description(&EnrollmentEvent::EnrollmentCompleted),
        ),
        EventKind::EnrollmentTokenAdded => enrollment(
            EventType::EnrollmentTokenAdded,
            serde_json::to_value(EnrollmentTokenMetadata {
                user: ctx.user.clone().into(),
            })
            .ok(),
            get_enrollment_event_description(&EnrollmentEvent::TokenAdded {
                user: ctx.user.clone(),
            }),
        ),
        EventKind::PasswordResetRequested => enrollment(
            EventType::PasswordResetRequested,
            None,
            get_enrollment_event_description(&EnrollmentEvent::PasswordResetRequested),
        ),
        EventKind::PasswordResetStarted => enrollment(
            EventType::PasswordResetStarted,
            None,
            get_enrollment_event_description(&EnrollmentEvent::PasswordResetStarted),
        ),
        EventKind::PasswordResetCompleted => enrollment(
            EventType::PasswordResetCompleted,
            None,
            get_enrollment_event_description(&EnrollmentEvent::PasswordResetCompleted),
        ),
        EventKind::VpnConnected => build_vpn_event(
            rng,
            ctx.device,
            ctx.locations,
            EventType::VpnClientConnected,
        ),
        EventKind::VpnDisconnected => build_vpn_event(
            rng,
            ctx.device,
            ctx.locations,
            EventType::VpnClientDisconnected,
        ),
        EventKind::VpnMfaConnected => build_vpn_event(
            rng,
            ctx.device,
            ctx.locations,
            EventType::VpnClientMfaConnected,
        ),
        EventKind::VpnMfaDisconnected => build_vpn_event(
            rng,
            ctx.device,
            ctx.locations,
            EventType::VpnClientMfaDisconnected,
        ),
        EventKind::VpnMfaSuccess => build_vpn_event(
            rng,
            ctx.device,
            ctx.locations,
            EventType::VpnClientMfaSuccess,
        ),
    }
}

fn build_network_device_event(
    rng: &mut ThreadRng,
    ctx: &BuildContext,
    user_agent: &str,
    event_type: EventType,
) -> GeneratedEvent {
    let location = ctx
        .locations
        .choose(rng)
        .expect("build_network_device_event called without any locations")
        .clone();
    let device = fabricate_network_device(rng, ctx.user.id);

    let (description, metadata) = match event_type {
        EventType::NetworkDeviceAdded => (
            get_defguard_event_description(&DefguardEvent::NetworkDeviceAdded {
                device: device.clone(),
                location: location.clone(),
            }),
            serde_json::to_value(NetworkDeviceMetadata {
                device,
                location: location.clone(),
            })
            .ok(),
        ),
        EventType::NetworkDeviceRemoved => (
            get_defguard_event_description(&DefguardEvent::NetworkDeviceRemoved {
                device: device.clone(),
                location: location.clone(),
            }),
            serde_json::to_value(NetworkDeviceMetadata {
                device,
                location: location.clone(),
            })
            .ok(),
        ),
        _ => unreachable!("build_network_device_event called with a non-network event type"),
    };

    GeneratedEvent {
        module: ActivityLogModule::Defguard,
        event_type,
        description,
        metadata,
        location: Some(location.name),
        device: user_agent.to_string(),
    }
}

fn build_vpn_event(
    rng: &mut ThreadRng,
    device: &Device<Id>,
    locations: &[WireguardNetwork<Id>],
    event_type: EventType,
) -> GeneratedEvent {
    let location = locations
        .choose(rng)
        .expect("build_vpn_event called without any locations")
        .clone();
    let device = device.clone();

    let location_name = Some(location.name.clone());
    let device_str = match event_type {
        EventType::VpnClientMfaSuccess => device.to_string(),
        _ => format!("{} (ID {})", device.name, device.id),
    };

    let (description, metadata) = match event_type {
        EventType::VpnClientConnected => (
            get_vpn_event_description(&VpnEvent::ConnectedToLocation {
                location: location.clone(),
                device: device.clone(),
            }),
            serde_json::to_value(VpnClientMetadata { location, device }).ok(),
        ),
        EventType::VpnClientDisconnected => (
            get_vpn_event_description(&VpnEvent::DisconnectedFromLocation {
                location: location.clone(),
                device: device.clone(),
            }),
            serde_json::to_value(VpnClientMetadata { location, device }).ok(),
        ),
        EventType::VpnClientMfaConnected => (
            get_vpn_event_description(&VpnEvent::MfaConnectedToLocation {
                location: location.clone(),
                device: device.clone(),
            }),
            serde_json::to_value(VpnClientMetadata { location, device }).ok(),
        ),
        EventType::VpnClientMfaDisconnected => (
            get_vpn_event_description(&VpnEvent::MfaDisconnectedFromLocation {
                location: location.clone(),
                device: device.clone(),
            }),
            serde_json::to_value(VpnClientMetadata { location, device }).ok(),
        ),
        EventType::VpnClientMfaSuccess => {
            let method = random_client_mfa_method(rng);
            (
                get_vpn_event_description(&VpnEvent::ClientMfaSuccess {
                    location: location.clone(),
                    device: device.clone(),
                    method,
                }),
                serde_json::to_value(VpnClientMfaMetadata {
                    location,
                    device,
                    method,
                    mobile_auth_device_name: None,
                })
                .ok(),
            )
        }
        _ => unreachable!("build_vpn_event called with a non-VPN event type"),
    };

    GeneratedEvent {
        module: ActivityLogModule::Vpn,
        event_type,
        description,
        metadata,
        location: location_name,
        device: device_str,
    }
}

fn fabricate_security_key(rng: &mut ThreadRng, user_id: Id) -> WebAuthn<Id> {
    WebAuthn {
        id: rng.gen_range(1_000..1_000_000),
        user_id,
        name: SECURITY_KEY_NAMES
            .choose(rng)
            .expect("SECURITY_KEY_NAMES is non-empty")
            .to_string(),
        passkey: Vec::new(),
    }
}

fn fabricate_network_device(rng: &mut ThreadRng, user_id: Id) -> Device<Id> {
    let mut device: Device = rng.r#gen();
    device.name = NETWORK_DEVICE_NAMES
        .choose(rng)
        .expect("NETWORK_DEVICE_NAMES is non-empty")
        .to_string();
    device.user_id = user_id;
    device.device_type = DeviceType::Network;
    device.description = None;
    device.with_id(rng.gen_range(1_000..1_000_000))
}

fn random_user_agent(rng: &mut ThreadRng) -> &'static str {
    USER_AGENTS.choose(rng).expect("USER_AGENTS is non-empty")
}

fn random_mfa_method(rng: &mut ThreadRng) -> MFAMethod {
    *[
        MFAMethod::OneTimePassword,
        MFAMethod::Webauthn,
        MFAMethod::Email,
    ]
    .choose(rng)
    .expect("slice is non-empty")
}

fn random_client_mfa_method(rng: &mut ThreadRng) -> ClientMFAMethod {
    *[
        ClientMFAMethod::Totp,
        ClientMFAMethod::Email,
        ClientMFAMethod::Biometric,
        ClientMFAMethod::MobileApprove,
    ]
    .choose(rng)
    .expect("slice is non-empty")
}
