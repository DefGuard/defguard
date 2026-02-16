pub mod activity_log_stream;
pub mod db;
pub mod directory_sync;
pub mod firewall;
pub mod grpc;
pub mod handlers;
pub mod ldap;
pub mod license;
pub mod limits;
pub mod snat;
mod utils;

use license::{get_cached_license, validate_license};
use limits::get_counts;

use crate::enterprise::license::LicenseTier;

/// Helper function to gate features which require a base license (Team or Business tier)
#[must_use]
pub fn is_business_license_active() -> bool {
    is_license_tier_active(LicenseTier::Business)
}

/// Helper function to gate features which require an Enterprise tier license
#[must_use]
pub fn is_enterprise_license_active() -> bool {
    is_license_tier_active(LicenseTier::Enterprise)
}

/// Shared logic for gating features to specific license tiers
fn is_license_tier_active(tier: LicenseTier) -> bool {
    debug!("Checking if features for {tier} license tier should be enabled");

    // get current object counts
    let counts = get_counts();

    let license = get_cached_license();
    let validation_result = validate_license(license.as_ref(), &counts, tier);
    debug!("License validation result: {validation_result:?}");
    validation_result.is_ok()
}

#[cfg(test)]
mod test {
    use chrono::{TimeDelta, Utc};

    use crate::{
        enterprise::{
            is_business_license_active, is_enterprise_license_active,
            license::{License, LicenseTier, set_cached_license},
            limits::{Counts, set_counts},
        },
        grpc::proto::enterprise::license::LicenseLimits,
    };

    #[test]
    fn test_feature_gates_no_license() {
        set_cached_license(None);

        let counts = Counts::new(1, 1, 1, 1);
        set_counts(counts);

        assert!(!is_business_license_active());
        assert!(!is_enterprise_license_active());
    }

    #[test]
    fn test_feature_gates_with_license() {
        // exceed free limits
        let counts = Counts::new(1, 1, 5, 1);
        set_counts(counts);

        // set Business license
        let users_limit = 15;
        let devices_limit = 35;
        let locations_limit = 5;
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
            LicenseTier::Business,
        );
        set_cached_license(Some(license));

        assert!(is_business_license_active());
        assert!(!is_enterprise_license_active());

        // set Enterprise license
        let users_limit = 15;
        let devices_limit = 35;
        let locations_limit = 5;
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
            LicenseTier::Enterprise,
        );
        set_cached_license(Some(license));

        assert!(is_business_license_active());
        assert!(is_enterprise_license_active());
    }
}
