use defguard_common::global_value;
use sqlx::{PgPool, error::Error as SqlxError, query};

use super::license::License;
#[cfg(test)]
use super::license::get_cached_license;

#[derive(Debug)]
#[cfg_attr(test, derive(Clone))]
pub struct Counts {
    user: u32,
    user_device: u32,
    network_device: u32,
    location: u32,
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
        location: result
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
            location: 0,
            network_device: 0,
        }
    }

    #[cfg(test)]
    pub(crate) fn new(user: u32, user_device: u32, location: u32, network_device: u32) -> Self {
        Self {
            user,
            user_device,
            network_device,
            location,
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
        } else {
            true
        }
    }

    pub(crate) fn user(&self) -> u32 {
        self.user
    }

    pub(crate) fn user_device(&self) -> u32 {
        self.user_device
    }

    pub(crate) fn network_device(&self) -> u32 {
        self.network_device
    }

    pub(crate) fn location(&self) -> u32 {
        self.location
    }

    pub(crate) fn is_over_license_limits(&self, license: &License) -> bool {
        let limits = &license.limits;
        match limits {
            Some(limits) => self.user > limits.users || self.location > limits.locations,
            // unlimited license
            None => false,
        }
    }
}

#[cfg(test)]
mod test {
    use chrono::{TimeDelta, Utc};

    use super::*;
    use crate::{
        enterprise::license::{License, LicenseTier, set_cached_license},
        grpc::proto::enterprise::license::LicenseLimits,
    };

    #[test]
    fn test_counts() {
        let counts = Counts {
            user: 1,
            user_device: 2,
            location: 3,
            network_device: 4,
        };

        set_counts(counts);

        let counts = get_counts();

        assert_eq!(counts.user, 1);
        assert_eq!(counts.user_device, 2);
        assert_eq!(counts.location, 3);
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
            false,
            None,
            Some(limits),
            None,
            LicenseTier::Business,
        );

        set_cached_license(Some(license));

        // User limit
        {
            let counts = Counts {
                user: users_limit + 1,
                user_device: 1,
                location: 1,
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
                location: locations_limit + 1,
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
                location: locations_limit,
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
                location: locations_limit + 1,
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
            LicenseTier::Business,
        );
        set_cached_license(Some(license));

        // it's not possible to be over the limit
        {
            let counts = Counts {
                user: u32::MAX,
                user_device: u32::MAX,
                location: u32::MAX,
                network_device: u32::MAX,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(!counts.is_over_limit());
        }
    }
}
