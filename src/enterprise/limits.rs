use sqlx::{error::Error as SqlxError, query, PgPool};
use std::sync::{RwLock, RwLockReadGuard};

#[cfg(test)]
use super::license::get_cached_license;
use super::license::License;

// Limits for free users
pub const DEFAULT_USERS_LIMIT: u32 = 5;
pub const DEFAULT_DEVICES_LIMIT: u32 = 10;
pub const DEFAULT_LOCATIONS_LIMIT: u32 = 1;

#[derive(Debug, Default)]
pub(crate) struct Counts {
    user: u32,
    device: u32,
    wireguard_network: u32,
}

static COUNTS: RwLock<Counts> = RwLock::new(Counts {
    user: 0,
    device: 0,
    wireguard_network: 0,
});

fn set_counts(new_counts: Counts) {
    *COUNTS
        .write()
        .expect("Failed to acquire lock on the enterprise limit counts.") = new_counts;
}

pub(crate) fn get_counts() -> RwLockReadGuard<'static, Counts> {
    COUNTS
        .read()
        .expect("Failed to acquire lock on the enterprise limit counts.")
}

/// Update the counts of users, devices, and wireguard networks stored in the memory.
// TODO: Use it with database triggers when they are implemented
pub async fn update_counts(pool: &PgPool) -> Result<(), SqlxError> {
    debug!("Updating device, user, and wireguard network counts.");
    let result = query!(
        "SELECT \
        (SELECT count(*) FROM \"user\") \"users!\", \
        (SELECT count(*) FROM device) \"devices!\", \
        (SELECT count(*) FROM wireguard_network) \"wireguard_networks!\"
        "
    )
    .fetch_one(pool)
    .await?;

    // do type conversion since Postgres does not support unsigned integers
    let counts = Counts {
        user: result
            .users
            .try_into()
            .expect("user count should never be negative"),
        device: result
            .devices
            .try_into()
            .expect("device count should never be negative"),
        wireguard_network: result
            .wireguard_networks
            .try_into()
            .expect("network count should never be negative"),
    };

    set_counts(counts);
    debug!(
        "Updated device, user, and wireguard network counts stored in memory, new counts: {:?}",
        get_counts()
    );

    Ok(())
}

pub async fn do_count_update(pool: &PgPool) -> Result<(), SqlxError> {
    update_counts(pool).await?;
    Ok(())
}

impl Counts {
    #[cfg(test)]
    pub(crate) fn new(user: u32, device: u32, wireguard_network: u32) -> Self {
        Self {
            user,
            device,
            wireguard_network,
        }
    }

    #[cfg(test)]
    pub(crate) fn is_over_limit(&self) -> bool {
        debug!("Checking if current object counts ({self:?}) exceed license limits");

        // fetch current license
        let maybe_license = get_cached_license();

        // validate limits against license if available, use defaults otherwise
        match &*maybe_license {
            Some(license) => {
                debug!("Cached license found. Validating license limits...");
                self.validate_license_limits(license)
            }
            // free tier
            None => {
                debug!("Cached license not found. Using default limits for validation...");
                self.user > DEFAULT_USERS_LIMIT
                    || self.device > DEFAULT_DEVICES_LIMIT
                    || self.wireguard_network > DEFAULT_LOCATIONS_LIMIT
            }
        }
    }

    pub(crate) fn validate_license_limits(&self, license: &License) -> bool {
        let limits = &license.limits;
        match limits {
            Some(limits) => {
                self.user > limits.users
                    || self.device > limits.devices
                    || self.wireguard_network > limits.locations
            }
            // unlimited license
            None => false,
        }
    }

    /// Checks if current object count exceeds default limits
    pub(crate) fn needs_enterprise_license(&self) -> bool {
        debug!("Checking if current object counts ({self:?}) exceed default limits");
        self.user > DEFAULT_USERS_LIMIT
            || self.device > DEFAULT_DEVICES_LIMIT
            || self.wireguard_network > DEFAULT_LOCATIONS_LIMIT
    }

    pub(crate) fn get_exceeded_limits(&self) -> LimitsExceeded {
        LimitsExceeded {
            user: self.user > DEFAULT_USERS_LIMIT,
            device: self.device > DEFAULT_DEVICES_LIMIT,
            wireguard_network: self.wireguard_network > DEFAULT_LOCATIONS_LIMIT,
        }
    }
}

// Granular exceeded limits info for the AppInfo endpoint.
#[derive(Serialize)]
pub(crate) struct LimitsExceeded {
    pub user: bool,
    pub device: bool,
    pub wireguard_network: bool,
}

/// Returns true if any of the limits has been exceeded.
impl LimitsExceeded {
    pub(crate) fn any(&self) -> bool {
        self.user || self.device || self.wireguard_network
    }
}

#[cfg(test)]
mod test {
    use chrono::{TimeDelta, Utc};

    use crate::enterprise::license::{set_cached_license, License, LicenseLimits};

    use super::*;

    #[test]
    fn test_counts() {
        let counts = Counts {
            user: 1,
            device: 2,
            wireguard_network: 3,
        };

        set_counts(counts);

        let counts = get_counts();

        assert_eq!(counts.user, 1);
        assert_eq!(counts.device, 2);
        assert_eq!(counts.wireguard_network, 3);
    }

    #[test]
    fn test_is_over_limit_free_tier() {
        // User limit
        {
            let counts = Counts {
                user: DEFAULT_USERS_LIMIT + 1,
                device: 1,
                wireguard_network: 1,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }

        // Device limit
        {
            let counts = Counts {
                user: 1,
                device: DEFAULT_DEVICES_LIMIT + 1,
                wireguard_network: 1,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }

        // Wireguard network limit
        {
            let counts = Counts {
                user: 1,
                device: 1,
                wireguard_network: DEFAULT_LOCATIONS_LIMIT + 1,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }

        // No limit
        {
            let counts = Counts {
                user: 1,
                device: 1,
                wireguard_network: 1,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(!counts.is_over_limit());
        }

        // All limits
        {
            let counts = Counts {
                user: DEFAULT_USERS_LIMIT + 1,
                device: DEFAULT_DEVICES_LIMIT,
                wireguard_network: DEFAULT_LOCATIONS_LIMIT,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }
    }

    #[test]
    fn test_is_over_limit_license_with_limits() {
        let users_limit = 15;
        let devices_limit = 35;
        let locations_limit = 4;

        let limits = LicenseLimits {
            users: users_limit,
            devices: devices_limit,
            locations: locations_limit,
        };
        let license = License::new(
            "test".to_string(),
            true,
            Some(Utc::now() + TimeDelta::days(1)),
            Some(limits),
        );
        set_cached_license(Some(license));

        // User limit
        {
            let counts = Counts {
                user: users_limit + 1,
                device: 1,
                wireguard_network: 1,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }

        // Device limit
        {
            let counts = Counts {
                user: 1,
                device: devices_limit + 1,
                wireguard_network: 1,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }

        // Wireguard network limit
        {
            let counts = Counts {
                user: 1,
                device: 1,
                wireguard_network: locations_limit + 1,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }

        // No limit
        {
            let counts = Counts {
                user: users_limit,
                device: devices_limit,
                wireguard_network: locations_limit,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(!counts.is_over_limit());
        }

        // All limits
        {
            let counts = Counts {
                user: users_limit + 1,
                device: devices_limit + 1,
                wireguard_network: locations_limit + 1,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }
    }

    #[test]
    fn test_is_over_limit_unlimited_license() {
        let license = License::new(
            "test".to_string(),
            true,
            Some(Utc::now() + TimeDelta::days(1)),
            None,
        );
        set_cached_license(Some(license));

        // it's not possible to be over the limit
        {
            let counts = Counts {
                user: u32::MAX,
                device: u32::MAX,
                wireguard_network: u32::MAX,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(!counts.is_over_limit());
        }
    }

    #[test]
    fn test_limits_exceeded() {
        let exceed_user = DEFAULT_DEVICES_LIMIT + 5;
        let exceed_device = DEFAULT_DEVICES_LIMIT + 5;
        let exceed_wireguard_network = DEFAULT_LOCATIONS_LIMIT + 5;

        let counts = Counts {
            user: exceed_user,
            device: 0,
            wireguard_network: 0,
        };
        set_counts(counts);
        let exceeded = get_counts().get_exceeded_limits();
        assert!(exceeded.user);
        assert!(!exceeded.device);
        assert!(!exceeded.wireguard_network);
        assert!(exceeded.any());

        let counts = Counts {
            user: 0,
            device: exceed_device,
            wireguard_network: 0,
        };
        set_counts(counts);
        let exceeded = get_counts().get_exceeded_limits();
        assert!(!exceeded.user);
        assert!(exceeded.device);
        assert!(!exceeded.wireguard_network);
        assert!(exceeded.any());

        let counts = Counts {
            user: 0,
            device: 0,
            wireguard_network: exceed_wireguard_network,
        };
        set_counts(counts);
        let exceeded = get_counts().get_exceeded_limits();
        assert!(!exceeded.user);
        assert!(!exceeded.device);
        assert!(exceeded.wireguard_network);
        assert!(exceeded.any());

        let counts = Counts {
            user: 0,
            device: 0,
            wireguard_network: 0,
        };
        set_counts(counts);
        let exceeded = get_counts().get_exceeded_limits();
        assert!(!exceeded.user);
        assert!(!exceeded.device);
        assert!(!exceeded.wireguard_network);
        assert!(!exceeded.any());
    }
}
