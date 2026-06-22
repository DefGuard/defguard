pub mod activity_log_stream;
pub mod allowed_ips;
pub mod db;
pub mod directory_sync;
pub mod firewall;
pub mod grpc;
pub mod handlers;
pub mod ldap;
pub mod license;
pub mod limits;
pub mod posture;
pub mod snat;
mod utils;

use license::{License, get_cached_license, validate_license};
use limits::get_counts;

pub use crate::enterprise::license::LicenseFeature;
use crate::enterprise::license::LicenseTier;

/// Returns whether a valid license grants the given feature, considering both the tier baseline
/// and any explicit additive flags. Does not check validity; call `validate_license` first.
pub(crate) fn license_grants_feature(license: &License, feature: LicenseFeature) -> bool {
    license.tier.included_features().contains(&feature) || license.has_feature(feature)
}

/// Helper function to gate features which require a base license (Team or Business tier)
#[must_use]
pub fn is_business_license_active() -> bool {
    is_license_tier_active(LicenseTier::Business)
}

/// Helper function to gate an enterprise feature.
///
/// Requires a valid base license (present, not past its maximum overdue date, within limits).
/// The Enterprise tier unlocks every feature. For lower tiers, passing `Some(feature)` also
/// unlocks that feature when it has been granted individually via an additive license feature
/// flag; passing `None` gates strictly on the Enterprise tier (no flag can satisfy it).
#[must_use]
pub fn is_enterprise_license_active(feature: Option<LicenseFeature>) -> bool {
    let counts = get_counts();
    let license = get_cached_license();
    let Some(license) = license.as_ref() else {
        return false;
    };
    if validate_license(Some(license), &counts, LicenseTier::Business).is_err() {
        return false;
    }
    match feature {
        Some(f) => license_grants_feature(license, f),
        None => license.tier == LicenseTier::Enterprise,
    }
}

/// Shared logic for gating features to specific license tiers
fn is_license_tier_active(tier: LicenseTier) -> bool {
    trace!("Checking if features for {tier} license tier should be enabled");

    // get current object counts
    let counts = get_counts();

    let license = get_cached_license();
    let validation_result = validate_license(license.as_ref(), &counts, tier);
    trace!("License validation result: {validation_result:?}");
    validation_result.is_ok()
}

#[cfg(test)]
mod test {
    use chrono::{TimeDelta, Utc};
    use strum::VariantArray;

    use crate::{
        enterprise::{
            LicenseFeature, is_business_license_active, is_enterprise_license_active,
            license::{License, LicenseTier, SupportType, set_cached_license},
            limits::{Counts, set_counts},
        },
        grpc::proto::enterprise::license::LicenseLimits,
    };

    fn license_limits() -> LicenseLimits {
        LicenseLimits {
            users: 15,
            devices: 35,
            locations: 5,
            network_devices: Some(10),
        }
    }

    #[test]
    fn test_feature_gates_no_license() {
        set_cached_license(None);

        let counts = Counts::new(1, 1, 1, 1);
        set_counts(counts);

        assert!(!is_business_license_active());
        for &feature in LicenseFeature::VARIANTS {
            assert!(!is_enterprise_license_active(Some(feature)));
        }
    }

    #[test]
    fn test_feature_gates_with_license() {
        // exceed free limits
        let counts = Counts::new(1, 1, 5, 1);
        set_counts(counts);

        // set Business license
        let license = License::new(
            "test".to_owned(),
            true,
            Some(Utc::now() + TimeDelta::days(1)),
            Some(license_limits()),
            None,
            LicenseTier::Business,
            SupportType::Basic,
            vec![],
        );
        set_cached_license(Some(license));

        assert!(is_business_license_active());
        for &feature in LicenseFeature::VARIANTS {
            assert!(!is_enterprise_license_active(Some(feature)));
        }

        // set Enterprise license
        let license = License::new(
            "test".to_owned(),
            true,
            Some(Utc::now() + TimeDelta::days(1)),
            Some(license_limits()),
            None,
            LicenseTier::Enterprise,
            SupportType::Basic,
            vec![],
        );
        set_cached_license(Some(license));

        assert!(is_business_license_active());
        for &feature in LicenseFeature::VARIANTS {
            assert!(is_enterprise_license_active(Some(feature)));
        }
    }

    #[test]
    fn test_additive_feature_flags() {
        let counts = Counts::new(1, 1, 5, 1);
        set_counts(counts);

        // a Business license that has been granted a single enterprise feature
        let license = License::new(
            "test".to_owned(),
            true,
            Some(Utc::now() + TimeDelta::days(1)),
            Some(license_limits()),
            None,
            LicenseTier::Business,
            SupportType::Basic,
            vec![LicenseFeature::DevicePosture],
        );
        set_cached_license(Some(license));

        // only the granted feature is unlocked, the rest stay disabled
        assert!(is_enterprise_license_active(Some(
            LicenseFeature::DevicePosture
        )));
        assert!(!is_enterprise_license_active(Some(
            LicenseFeature::ServiceLocations
        )));
        assert!(!is_enterprise_license_active(Some(
            LicenseFeature::AclAllowedIps
        )));
        assert!(!is_enterprise_license_active(Some(
            LicenseFeature::ComponentHa
        )));

        // a Business license without the flag never satisfies a tier-only (None) gate
        assert!(!is_enterprise_license_active(None));

        // flags don't survive exceeding the license limits
        let over_limit = Counts::new(100, 100, 100, 100);
        set_counts(over_limit);
        assert!(!is_enterprise_license_active(Some(
            LicenseFeature::DevicePosture
        )));
    }
}
