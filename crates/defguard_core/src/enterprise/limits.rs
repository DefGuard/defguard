use sqlx::{PgPool, error::Error as SqlxError, query};

use super::license::License;
#[cfg(test)]
use super::license::get_cached_license;
use crate::{global_value, grpc::proto::enterprise::license::LicenseLimits};

// Limits for free users
pub const DEFAULT_USERS_LIMIT: u32 = 5;
pub const DEFAULT_DEVICES_LIMIT: u32 = 10;
pub const DEFAULT_LOCATIONS_LIMIT: u32 = 1;
pub const DEFAULT_NETWORK_DEVICES_LIMIT: u32 = 10;

#[derive(Debug)]
#[cfg_attr(test, derive(Clone))]
pub struct Counts {
    user: u32,
    user_device: u32,
    network_device: u32,
    wireguard_network: u32,
}

global_value!(COUNTS, Counts, Counts::default(), set_counts, get_counts);

/// Update the counts of users, devices, and wireguard networks stored in the memory.
// TODO: Use it with database triggers when they are implemented
pub async fn update_counts<'e, E: sqlx::PgExecutor<'e>>(executor: E) -> Result<(), SqlxError> {
    debug!("Updating device, user, and wireguard network counts.");
    let result = query!(
        "SELECT \
        (SELECT count(*) FROM \"user\") \"users!\", \
        (SELECT count(*) FROM device WHERE device_type = 'user') \"user_devices!\", \
        (SELECT count(*) FROM device WHERE device_type = 'network') \"network_devices!\",
        (SELECT count(*) FROM wireguard_network) \"wireguard_networks!\"
        "
    )
    .fetch_one(executor)
    .await?;

    // do type conversion since Postgres does not support unsigned integers
    let counts = Counts {
        user: result
            .users
            .try_into()
            .expect("user count should never be negative"),
        user_device: result
            .user_devices
            .try_into()
            .expect("device count should never be negative"),
        network_device: result
            .network_devices
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
    pub(crate) const fn default() -> Self {
        Self {
            user: 0,
            user_device: 0,
            wireguard_network: 0,
            network_device: 0,
        }
    }

    #[cfg(test)]
    pub(crate) fn new(
        user: u32,
        user_device: u32,
        wireguard_network: u32,
        network_device: u32,
    ) -> Self {
        Self {
            user,
            user_device,
            wireguard_network,
            network_device,
        }
    }

    #[cfg(test)]
    pub(crate) fn is_over_limit(&self) -> bool {
        debug!("Checking if current object counts ({self:?}) exceed license limits");

        // fetch current license
        let maybe_license = get_cached_license();

        // validate limits against license if available, use defaults otherwise
        if let Some(license) = maybe_license.as_ref() {
            debug!("Cached license found. Validating license limits...");
            self.is_over_license_limits(license)
        }
        // free tier
        else {
            debug!("Cached license not found. Using default limits for validation...");
            self.user > DEFAULT_USERS_LIMIT
                || self.user_device > DEFAULT_DEVICES_LIMIT
                || self.wireguard_network > DEFAULT_LOCATIONS_LIMIT
        }
    }

    // New linceses have a network device limit field, this function handles backwards compatibility
    // If no such field is present = old behavior (user devices + network devices <= devices limit)
    // If field is present, check user devices and network devices separately
    fn is_over_device_limit(&self, limits: &LicenseLimits) -> bool {
        match limits.network_devices {
            Some(devices) => self.user_device > limits.devices || self.network_device > devices,
            None => self.user_device + self.network_device > limits.devices,
        }
    }

    pub(crate) fn is_over_license_limits(&self, license: &License) -> bool {
        let limits = &license.limits;
        match limits {
            Some(limits) => {
                self.user > limits.users
                    || self.is_over_device_limit(limits)
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
            || self.user_device > DEFAULT_DEVICES_LIMIT
            || self.wireguard_network > DEFAULT_LOCATIONS_LIMIT
    }

    pub(crate) fn get_exceeded_limits(&self, license: Option<&License>) -> LimitsExceeded {
        if let Some(license) = license {
            if let Some(limits) = &license.limits {
                LimitsExceeded {
                    user: self.user > limits.users,
                    device: self.user_device > limits.devices,
                    wireguard_network: self.wireguard_network > limits.locations,
                    network_device: match limits.network_devices {
                        Some(devices) => self.network_device > devices,
                        None => false,
                    },
                }
            } else {
                LimitsExceeded {
                    user: false,
                    device: false,
                    wireguard_network: false,
                    network_device: false,
                }
            }
        } else {
            LimitsExceeded {
                user: self.user > DEFAULT_DEVICES_LIMIT,
                device: self.user_device > DEFAULT_DEVICES_LIMIT,
                wireguard_network: self.wireguard_network > DEFAULT_LOCATIONS_LIMIT,
                network_device: self.network_device > DEFAULT_NETWORK_DEVICES_LIMIT,
            }
        }
    }
}

// Granular exceeded limits info for the AppInfo endpoint.
#[derive(Serialize)]
pub(crate) struct LimitsExceeded {
    pub user: bool,
    pub device: bool,
    pub wireguard_network: bool,
    pub network_device: bool,
}

/// Returns true if any of the limits has been exceeded.
impl LimitsExceeded {
    pub(crate) fn any(&self) -> bool {
        self.user || self.device || self.wireguard_network || self.network_device
    }
}

#[cfg(test)]
mod test {
    use chrono::{TimeDelta, Utc};

    use super::*;
    use crate::{
        enterprise::license::{License, set_cached_license},
        grpc::proto::enterprise::license::LicenseLimits,
    };

    #[test]
    fn test_network_device_limit_old_license() {
        let limits = LicenseLimits {
            users: 10,
            devices: 20,
            locations: 5,
            network_devices: None,
        };
        let counts = Counts {
            user: 5,
            user_device: 15,
            wireguard_network: 3,
            network_device: 6,
        };
        assert!(counts.is_over_device_limit(&limits));

        let counts = Counts {
            user: 5,
            user_device: 10,
            wireguard_network: 3,
            network_device: 5,
        };
        assert!(!counts.is_over_device_limit(&limits));

        let limits = LicenseLimits {
            users: 10,
            devices: 20,
            locations: 5,
            network_devices: Some(10),
        };

        let counts = Counts {
            user: 5,
            user_device: 15,
            wireguard_network: 3,
            network_device: 6,
        };
        assert!(!counts.is_over_device_limit(&limits));

        let counts = Counts {
            user: 5,
            user_device: 15,
            wireguard_network: 3,
            network_device: 11,
        };
        assert!(counts.is_over_device_limit(&limits));
    }

    #[test]
    fn test_counts() {
        let counts = Counts {
            user: 1,
            user_device: 2,
            wireguard_network: 3,
            network_device: 4,
        };

        set_counts(counts);

        let counts = get_counts();

        assert_eq!(counts.user, 1);
        assert_eq!(counts.user_device, 2);
        assert_eq!(counts.wireguard_network, 3);
    }

    #[test]
    fn test_is_over_limit_free_tier() {
        // User limit
        {
            let counts = Counts {
                user: DEFAULT_USERS_LIMIT + 1,
                user_device: 1,
                wireguard_network: 1,
                network_device: 1,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }

        // Device limit
        {
            let counts = Counts {
                user: 1,
                user_device: DEFAULT_DEVICES_LIMIT + 1,
                wireguard_network: 1,
                network_device: 1,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }

        // Wireguard network limit
        {
            let counts = Counts {
                user: 1,
                user_device: 1,
                wireguard_network: DEFAULT_LOCATIONS_LIMIT + 1,
                network_device: 1,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }

        // No limit
        {
            let counts = Counts {
                user: 1,
                user_device: 1,
                wireguard_network: 1,
                network_device: 1,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(!counts.is_over_limit());
        }

        // All limits
        {
            let counts = Counts {
                user: DEFAULT_USERS_LIMIT + 1,
                user_device: DEFAULT_DEVICES_LIMIT,
                wireguard_network: DEFAULT_LOCATIONS_LIMIT,
                network_device: 1,
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
        let network_devices_limit = 10;

        let limits = LicenseLimits {
            users: users_limit,
            devices: devices_limit,
            locations: locations_limit,
            network_devices: Some(network_devices_limit),
        };
        let license = License::new(
            "test".to_string(),
            true,
            Some(Utc::now() + TimeDelta::days(1)),
            Some(limits),
            None,
        );
        set_cached_license(Some(license));

        // User limit
        {
            let counts = Counts {
                user: users_limit + 1,
                user_device: 1,
                wireguard_network: 1,
                network_device: 1,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }

        // Device limit
        {
            let counts = Counts {
                user: 1,
                user_device: devices_limit + 1,
                wireguard_network: 1,
                network_device: 1,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }

        // Wireguard network limit
        {
            let counts = Counts {
                user: 1,
                user_device: 1,
                wireguard_network: locations_limit + 1,
                network_device: 1,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }

        // No limit
        {
            let counts = Counts {
                user: users_limit,
                user_device: devices_limit,
                wireguard_network: locations_limit,
                network_device: network_devices_limit,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(!counts.is_over_limit());
        }

        // All limits
        {
            let counts = Counts {
                user: users_limit + 1,
                user_device: devices_limit + 1,
                wireguard_network: locations_limit + 1,
                network_device: network_devices_limit + 1,
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
            None,
        );
        set_cached_license(Some(license));

        // it's not possible to be over the limit
        {
            let counts = Counts {
                user: u32::MAX,
                user_device: u32::MAX,
                wireguard_network: u32::MAX,
                network_device: u32::MAX,
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
        let exceed_network_device = DEFAULT_NETWORK_DEVICES_LIMIT + 5;

        let counts = Counts {
            user: exceed_user,
            user_device: 0,
            wireguard_network: 0,
            network_device: 0,
        };
        set_counts(counts);
        let exceeded = get_counts().get_exceeded_limits(None);
        assert!(exceeded.user);
        assert!(!exceeded.device);
        assert!(!exceeded.wireguard_network);
        assert!(!exceeded.network_device);
        assert!(exceeded.any());

        let counts = Counts {
            user: 0,
            user_device: exceed_device,
            wireguard_network: 0,
            network_device: 0,
        };
        set_counts(counts);
        let exceeded = get_counts().get_exceeded_limits(None);
        assert!(!exceeded.user);
        assert!(exceeded.device);
        assert!(!exceeded.wireguard_network);
        assert!(!exceeded.network_device);
        assert!(exceeded.any());

        let counts = Counts {
            user: 0,
            user_device: 0,
            wireguard_network: exceed_wireguard_network,
            network_device: 0,
        };
        set_counts(counts);
        let exceeded = get_counts().get_exceeded_limits(None);
        assert!(!exceeded.user);
        assert!(!exceeded.device);
        assert!(exceeded.wireguard_network);
        assert!(exceeded.any());

        let counts = Counts {
            user: 0,
            user_device: 0,
            wireguard_network: 0,
            network_device: exceed_network_device,
        };

        set_counts(counts);
        let exceeded = get_counts().get_exceeded_limits(None);
        assert!(!exceeded.user);
        assert!(!exceeded.device);
        assert!(!exceeded.wireguard_network);
        assert!(exceeded.network_device);
        assert!(exceeded.any());

        let counts = Counts {
            user: 0,
            user_device: 0,
            wireguard_network: 0,
            network_device: 0,
        };
        set_counts(counts);
        let exceeded = get_counts().get_exceeded_limits(None);
        assert!(!exceeded.user);
        assert!(!exceeded.device);
        assert!(!exceeded.wireguard_network);
        assert!(!exceeded.network_device);
        assert!(!exceeded.any());

        let license = License::new(
            "test".to_string(),
            true,
            Some(Utc::now() + TimeDelta::days(1)),
            Some(LicenseLimits {
                users: 2,
                devices: 2,
                locations: 2,
                network_devices: Some(2),
            }),
            None,
        );
        let counts = Counts {
            user: 3,
            user_device: 3,
            wireguard_network: 3,
            network_device: 3,
        };
        set_counts(counts);
        let exceeded = get_counts().get_exceeded_limits(Some(&license));
        assert!(exceeded.user);
        assert!(exceeded.device);
        assert!(exceeded.wireguard_network);
        assert!(exceeded.network_device);
        assert!(exceeded.any());

        let license = License::new(
            "test".to_string(),
            true,
            Some(Utc::now() + TimeDelta::days(1)),
            None,
            None,
        );
        let counts = Counts {
            user: 300,
            user_device: 300,
            wireguard_network: 300,
            network_device: 300,
        };
        set_counts(counts);
        let exceeded = get_counts().get_exceeded_limits(Some(&license));
        assert!(!exceeded.user);
        assert!(!exceeded.device);
        assert!(!exceeded.wireguard_network);
        assert!(!exceeded.network_device);
        assert!(!exceeded.any());
    }
}
