use sqlx::{error::Error as SqlxError, query, PgPool};
use std::sync::{RwLock, RwLockReadGuard};

use super::license::get_cached_license;

// Limits for free users
pub const DEFAULT_USERS_LIMIT: u32 = 5;
pub const DEFAULT_DEVICES_LIMIT: u32 = 10;
pub const DEFAULT_LOCATIONS_LIMIT: u32 = 1;

#[derive(Debug)]
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
// TODO: count network devices once implemented
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
    pub(crate) fn is_over_limit(&self) -> bool {
        debug!("Checking if current object counts ({self:?}) exceed license limits");

        // fetch current license
        let maybe_license = get_cached_license();

        // validate limits against license if available, use defaults otherwise
        match &*maybe_license {
            Some(license) => {
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
            // free tier
            None => {
                self.user > DEFAULT_USERS_LIMIT
                    || self.device > DEFAULT_DEVICES_LIMIT
                    || self.wireguard_network > DEFAULT_LOCATIONS_LIMIT
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::u32;

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
                user: 6,
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
                device: 11,
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
                wireguard_network: 2,
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
                user: 6,
                device: 11,
                wireguard_network: 2,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }
    }

    #[test]
    fn test_is_over_limit_license_with_limits() {
        let limits = LicenseLimits {
            users: 15,
            devices: 35,
            locations: 4,
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
                user: 16,
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
                device: 36,
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
                wireguard_network: 5,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }

        // No limit
        {
            let counts = Counts {
                user: 15,
                device: 35,
                wireguard_network: 4,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(!counts.is_over_limit());
        }

        // All limits
        {
            let counts = Counts {
                user: 16,
                device: 36,
                wireguard_network: 5,
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
}
