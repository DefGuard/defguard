use chrono::TimeZone;
use defguard_common::db::{
    models::{User, WireguardNetwork},
    setup_pool,
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::*;

#[test]
fn test_license() {
    let license = "CjAKIDBjNGRjYjU0MDA1NDRkNDdhZDg2MTdmY2RmMjcwNGNiGOLBtbsGIgYIChBkGAUStQGIswQAAQgAHRYhBJouPBfibqMI7c3KmaiEbAECmoSEBQJnd9BYAAoJEKiEbAECmoSEtuMEAJu+mQlHt+OsIb3DSiknwyB+Z3d/AtvaOxIrnGSgnpJ22jAwKTRfBrOJsJQr0dA9wB4yawbXGv6+m35QPABQdSM+clq7x5J2bxyhLla00O7cdf2BcdYmyBEv1D/ZIjT1XBFoYEXzwxniviNsw4ZJaRsRIylr7eWsTw1tu+8IF4/U";
    let license = License::from_base64(license).unwrap();
    assert_eq!(license.customer_id, "0c4dcb5400544d47ad8617fcdf2704cb");
    assert!(!license.subscription);
    assert_eq!(
        license.valid_until.unwrap(),
        Utc.with_ymd_and_hms(2024, 12, 26, 13, 57, 54).unwrap()
    );
    assert!(license.is_expired());

    let limits = license.limits.unwrap();
    assert_eq!(limits.users, 10);
    assert_eq!(limits.devices, 100);
    assert_eq!(limits.locations, 5);

    // pre-1.6 license defaults to Business tier
    assert_eq!(license.tier, LicenseTier::Business);
}

#[test]
fn test_map_license_features() {
    // known protobuf values map through to the domain enum, preserving order
    assert_eq!(
        map_license_features(&[1, 2, 3, 4]),
        vec![
            LicenseFeature::ServiceLocations,
            LicenseFeature::DevicePosture,
            LicenseFeature::AclAllowedIps,
            LicenseFeature::ComponentHa,
        ]
    );

    // unspecified (0) and unknown future values are dropped, recognized ones kept
    assert_eq!(
        map_license_features(&[0, 2, 99]),
        vec![LicenseFeature::DevicePosture]
    );

    assert!(map_license_features(&[]).is_empty());
}

// Real signed keys (verified against the embedded test public key) carrying additive
// feature flags on a Business tier. Guards that the on-the-wire feature bytes decode to
// the expected domain enum, end to end through signature verification.
#[test]
fn test_license_feature_flags() {
    // Business tier granting service locations, device posture and ACL-derived AllowedIPs.
    let key = "CjsKIGIzZmY0ZWY2YWMxYjQwMDI5MTM4MmI4YzM2ODkzZmIyEAEYr4my0wYiBggZEEsYAjABOAJCAwECAxLRAYjPBAABCAA5FiEEmi48F+JuowjtzcqZqIRsAQKahIQFAmo1Hl8bFIAAAAAABAAObWFudTIsMi41KzEuMTIsMCwzAAoJEKiEbAECmoSEYzIEAJnS/zaxyvGGXo0HZqxaAabXXm8OJ0wBfPjAqBjgCZ0WfrP4dpEY93C7TnEWm9Ry2JuzhuvETghUvZ5NFwdM9rMaK6Or74yPDyWyrkFDPLvgQvQIMEBeZzw/8TuTq1yUfkaN837BahG5v7R/4Z9vF+liau3Aoh+dpAs0mRXMu1Z2";
    let license = License::from_base64(key).unwrap();
    assert_eq!(license.customer_id, "b3ff4ef6ac1b400291382b8c36893fb2");
    assert_eq!(license.tier, LicenseTier::Business);
    assert_eq!(
        license.features,
        vec![
            LicenseFeature::ServiceLocations,
            LicenseFeature::DevicePosture,
            LicenseFeature::AclAllowedIps,
        ]
    );
    assert!(license.has_feature(LicenseFeature::ServiceLocations));
    assert!(!license.has_feature(LicenseFeature::ComponentHa));

    // Business tier granting only component HA.
    let key = "CjkKIGIzZmY0ZWY2YWMxYjQwMDI5MTM4MmI4YzM2ODkzZmIyEAEYr4my0wYiBggZEEsYAjABOAJCAQQS0QGIzwQAAQgAORYhBJouPBfibqMI7c3KmaiEbAECmoSEBQJqNSLbGxSAAAAAAAQADm1hbnUyLDIuNSsxLjEyLDAsMwAKCRCohGwBApqEhHzHBACC/YGBSIEIV/AIU/EyLOAJHIPp1bQJQ1UXYVY79228x3z2JY3nblcXUEP/O5VJrVEv2FxiO8k5vyF2XtQr38CSu2vUxA29jn6bu23tfUb9+T9uXAOkkfNGiidy1iDTJQ+aXi2N3B75toRauDE+4SqEDN7ffxKOlIJLjYGyl8oysg==";
    let license = License::from_base64(key).unwrap();
    assert_eq!(license.tier, LicenseTier::Business);
    assert_eq!(license.features, vec![LicenseFeature::ComponentHa]);
    assert!(license.has_feature(LicenseFeature::ComponentHa));
    assert!(!license.has_feature(LicenseFeature::DevicePosture));

    // Business tier with no feature flags.
    let key = "CjYKIGIzZmY0ZWY2YWMxYjQwMDI5MTM4MmI4YzM2ODkzZmIyEAEYr4my0wYiBggZEEsYAjABOAIS0QGIzwQAAQgAORYhBJouPBfibqMI7c3KmaiEbAECmoSEBQJqNR56GxSAAAAAAAQADm1hbnUyLDIuNSsxLjEyLDAsMwAKCRCohGwBApqEhHb1A/9b4goZRHbMpf1WDRpV/EflWs9I/X5IAbX+U2hyyNEkC1zS4CxdKUNaDKJftbOP4uTqb0zjbf1r51Lr00LwVTkJMypZfVvpW6uzwMuRwyxYeQRE/iBXLNoRsf2tLV5pwnUWBLQ6y33xf0fypDZQWi7C9gQ6lEkkTXgDq4BoOVZkgg==";
    let license = License::from_base64(key).unwrap();
    assert_eq!(license.tier, LicenseTier::Business);
    assert!(license.features.is_empty());
    assert!(!license.has_feature(LicenseFeature::ServiceLocations));
}

#[test]
fn test_legacy_license() {
    // use license key generated before user/device/location limits were introduced
    let license = "CigKIDVhMGRhZDRiOWNmZTRiNzZiYjkzYmI1Y2Q5MGM2ZjdjGNaw1LsGErUBiLMEAAEIAB0WIQSaLjwX4m6jCO3NypmohGwBApqEhAUCZ3fBjAAKCRCohGwBApqEhNX+A/9dQmucvCTm5ll9h7a8f1N7d7dAOQW8/xhVA4bZP3GATIya/RxZ+cp+oHRYvHwSiRG3smGbRzti9DdHaTC/X1nqjMvZ6M4pR+aBayFH7fSUQKRj5z40juZ/HTCH/236YG3IzUZmIasLYl8Em9AY3oobkkwh1Yw+v8XYaBTUsrOv9w==";
    let license = License::from_base64(license).unwrap();
    assert_eq!(license.customer_id, "5a0dad4b9cfe4b76bb93bb5cd90c6f7c");
    assert!(!license.subscription);
    assert_eq!(
        license.valid_until.unwrap(),
        Utc.with_ymd_and_hms(2025, 1, 1, 10, 26, 30).unwrap()
    );

    assert!(license.is_expired());

    // legacy license is unlimited
    assert!(license.limits.is_none());

    // legacy license defaults to Business tier
    assert_eq!(license.tier, LicenseTier::Business);
}

#[test]
fn test_new_license() {
    // This key has an additional test_field in the metadata that doesn't exist in the proto definition
    // It should still be able to decode the license correctly
    let license = "CjAKIDBjNGRjYjU0MDA1NDRkNDdhZDg2MTdmY2RmMjcwNGNiGOLBtbsGIgYIChBkGAUStQGIswQAAQgAHRYhBJouPBfibqMI7c3KmaiEbAECmoSEBQJnd9EMAAoJEKiEbAECmoSE/0kEAIb18pVTEYWQo0w6813nShJqi7++Uo/fX4pxaAzEiG9r5HGpZSbsceCarMiK1rBr93HOIMeDRsbZmJBA/MAYGi32uXgzLE8fGSd4lcUPAbpvlj7KNvQNH6sMelzQVw+AJVY+IASqO84nfy92taEVagbLqIwl/eSQUnehJBS+B5/z";
    let license = License::from_base64(license).unwrap();

    assert_eq!(license.customer_id, "0c4dcb5400544d47ad8617fcdf2704cb");
    assert!(!license.subscription);
    assert_eq!(
        license.valid_until.unwrap(),
        Utc.with_ymd_and_hms(2024, 12, 26, 13, 57, 54).unwrap()
    );

    // pre-1.6 license defaults to Business tier
    assert_eq!(license.tier, LicenseTier::Business);
}

#[test]
fn test_invalid_license() {
    let license = "CigKIDBjNGRjYjU0MDA1NDRkNDdhZDg2MTdmY2RmMjcwNGNiGOLBtbsGErUBiLMEAAEIAB0WIQSaLjwX4m6jCO3NypmohGwBApqEhAUCZ3ZjywAKCRCohGwBApqEhEwFBACpHDnIszU2+KZcGhi3kycd3a12PyXJuFhhY4cuSyC8YEND85BplSWK1L8nu5ghFULFlddXP9HTHdxhJbtx4SgOQ8pxUY3+OpBN4rfJOMF61tvMRLaWlz7FWm/RnHe8cpoAOYm4oKRS0+FA2qLThxSsVa+S907ty19c6mcDgi6V5g==";
    let license = License::from_base64(license).unwrap();
    let counts = Counts::default();
    assert!(license.validate(&counts, LicenseTier::Business).is_err());

    // One day past the expiry date, non-subscription license
    let license = License::new(
        "test".to_owned(),
        false,
        Some(Utc::now() - TimeDelta::days(1)),
        None,
        None,
        LicenseTier::Business,
        SupportType::Basic,
        Vec::new(),
    );
    assert!(license.validate(&counts, LicenseTier::Business).is_err());

    // One day before the expiry date, non-subscription license
    let license = License::new(
        "test".to_owned(),
        false,
        Some(Utc::now() + TimeDelta::days(1)),
        None,
        None,
        LicenseTier::Business,
        SupportType::Basic,
        Vec::new(),
    );
    assert!(license.validate(&counts, LicenseTier::Business).is_ok());

    // No expiry date, non-subscription license
    let license = License::new(
        "test".to_owned(),
        false,
        None,
        None,
        None,
        LicenseTier::Business,
        SupportType::Basic,
        Vec::new(),
    );
    assert!(license.validate(&counts, LicenseTier::Business).is_ok());

    // One day past the maximum overdue date
    let license = License::new(
        "test".to_owned(),
        true,
        Some(Utc::now() - MAX_OVERDUE_TIME - TimeDelta::days(1)),
        None,
        None,
        LicenseTier::Business,
        SupportType::Basic,
        Vec::new(),
    );
    assert!(license.validate(&counts, LicenseTier::Business).is_err());

    // One day before the maximum overdue date
    let license = License::new(
        "test".to_owned(),
        true,
        Some(Utc::now() - MAX_OVERDUE_TIME + TimeDelta::days(1)),
        None,
        None,
        LicenseTier::Business,
        SupportType::Basic,
        Vec::new(),
    );
    assert!(license.validate(&counts, LicenseTier::Business).is_ok());

    let counts = Counts::new(5, 5, 5, 5);

    // Over object count limits
    let license = License::new(
        "test".to_owned(),
        true,
        Some(Utc::now() - MAX_OVERDUE_TIME + TimeDelta::days(1)),
        Some(LicenseLimits {
            users: 1,
            devices: 1,
            locations: 1,
            network_devices: Some(1),
        }),
        None,
        LicenseTier::Business,
        SupportType::Basic,
        Vec::new(),
    );
    assert!(license.validate(&counts, LicenseTier::Business).is_err());

    // Below object count limits
    let license = License::new(
        "test".to_owned(),
        true,
        Some(Utc::now() - MAX_OVERDUE_TIME + TimeDelta::days(1)),
        Some(LicenseLimits {
            users: 10,
            devices: 10,
            locations: 10,
            network_devices: Some(10),
        }),
        None,
        LicenseTier::Business,
        SupportType::Basic,
        Vec::new(),
    );
    assert!(license.validate(&counts, LicenseTier::Business).is_ok());
}

#[test]
fn test_license_tiers() {
    let legacy_license = "CjAKIDBjNGRjYjU0MDA1NDRkNDdhZDg2MTdmY2RmMjcwNGNiGOLBtbsGIgYIChBkGAUStQGIswQAAQgAHRYhBJouPBfibqMI7c3KmaiEbAECmoSEBQJnd9EMAAoJEKiEbAECmoSE/0kEAIb18pVTEYWQo0w6813nShJqi7++Uo/fX4pxaAzEiG9r5HGpZSbsceCarMiK1rBr93HOIMeDRsbZmJBA/MAYGi32uXgzLE8fGSd4lcUPAbpvlj7KNvQNH6sMelzQVw+AJVY+IASqO84nfy92taEVagbLqIwl/eSQUnehJBS+B5/z";
    let legacy_license = License::from_base64(legacy_license).unwrap();
    assert_eq!(legacy_license.tier, LicenseTier::Business);

    let business_license = "Ci4KJGEyYjE1M2MzLWYwZmEtNGUzNC05ZThkLWY0Nzk1NTA4OWMwNRiI7KTKBjABErUBiLMEAAEIAB0WIQSaLjwX4m6jCO3NypmohGwBApqEhAUCaT/7iAAKCRCohGwBApqEhHdaA/0QqDNiryYSzWTEayBMwEBE6KAxTEtwRzXOxQxsnULjbQMol/SRjqfu8iwlI4IeBQP3CuAR9kglewvwg3osXDldIns46W/cDBd0jxANebLY9SPz0JS6pStMnSzhZ6rFW5ns3nCz86EOyAA9npx0/qxHCbtT6Qzi//5JYQe6VvvCmw==";
    let business_license = License::from_base64(business_license).unwrap();
    assert_eq!(business_license.tier, LicenseTier::Business);

    let enterprise_license = "Ci4KJDRiYjMzZTUyLWUzNGMtNGQyMS1iNDVhLTkxY2EzYTMzNGMwORiy7KTKBjACErUBiLMEAAEIAB0WIQSaLjwX4m6jCO3NypmohGwBApqEhAUCaT/7sgAKCRCohGwBApqEhIMzBACGd7vIyLaRVGV/MAD8bpgWURG1x1tlxD9ehaSNkk01GkfZc+6+QwiTUBUOSp0MKPtuLmow5AIRKS9M75CQQ4bGtjLWO5cXJm1sduRpTvXwPLXNkRFPSxhjHmo4yjFFHMHMySqQE2WUjcz/b5dMT/WNqWYg7tSfT72eiK18eSVFTA==";
    let enterprise_license = License::from_base64(enterprise_license).unwrap();
    assert_eq!(enterprise_license.tier, LicenseTier::Enterprise);
}

#[test]
fn test_tier_included_features() {
    assert_eq!(
        LicenseTier::Enterprise.included_features(),
        LicenseFeature::VARIANTS,
    );
    assert!(LicenseTier::Business.included_features().is_empty());
}

#[test]
fn test_support_type_mapping() {
    assert_eq!(
        SupportType::try_from(SupportTypeProto::Unspecified).unwrap(),
        SupportType::Free
    );
    assert_eq!(
        SupportType::try_from(SupportTypeProto::Free).unwrap(),
        SupportType::Free
    );
    assert_eq!(
        SupportType::try_from(SupportTypeProto::Basic).unwrap(),
        SupportType::Basic
    );
    assert_eq!(
        SupportType::try_from(SupportTypeProto::Direct).unwrap(),
        SupportType::Direct
    );
    assert_eq!(
        SupportType::try_from(SupportTypeProto::BasicEnterprise).unwrap(),
        SupportType::BasicEnterprise
    );
    assert_eq!(
        SupportType::try_from(SupportTypeProto::DirectEnterprise).unwrap(),
        SupportType::DirectEnterprise
    );
}

#[test]
fn test_license_tier_mapping() {
    assert_eq!(
        LicenseTier::try_from(LicenseTierProto::Unspecified).unwrap(),
        LicenseTier::Business
    );
    assert_eq!(
        LicenseTier::try_from(LicenseTierProto::Business).unwrap(),
        LicenseTier::Business
    );
    assert_eq!(
        LicenseTier::try_from(LicenseTierProto::Enterprise).unwrap(),
        LicenseTier::Enterprise
    );
}

#[sqlx::test]
async fn test_trim_gateways_and_edges(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let location = WireguardNetwork::default().save(&pool).await.unwrap();
    let user = User::new(
        "tester",
        Some("hunter2"),
        "Tes",
        "Ter",
        "email@email.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();
    let fullname = user.fullname();

    Gateway::new(location.id, "Gateway 1", "localhost", 8000, &fullname)
        .save(&pool)
        .await
        .unwrap();
    Gateway::new(location.id, "Gateway 2", "localhost", 8001, &fullname)
        .save(&pool)
        .await
        .unwrap();

    Proxy::new("Proxy 1", "localhost", 9000, &fullname)
        .save(&pool)
        .await
        .unwrap();
    Proxy::new("Proxy 2", "localhost", 9001, &fullname)
        .save(&pool)
        .await
        .unwrap();

    let (proxy_control_tx, mut proxy_control_rx) =
        tokio::sync::mpsc::channel::<ProxyControlMessage>(8);

    trim_gateways_and_edges(&pool, &proxy_control_tx)
        .await
        .unwrap();

    let all_gateways = Gateway::all(&pool).await.unwrap();
    assert_eq!(1, all_gateways.iter().filter(|gw| gw.enabled).count());
    assert_eq!(1, all_gateways.iter().filter(|gw| !gw.enabled).count());

    let all_proxies = Proxy::all(&pool).await.unwrap();
    assert_eq!(1, all_proxies.iter().filter(|gw| gw.enabled).count());
    assert_eq!(1, all_proxies.iter().filter(|gw| !gw.enabled).count());

    // Only one Proxy has to be shut down.
    assert!(proxy_control_rx.try_recv().is_ok());
    assert!(proxy_control_rx.try_recv().is_err());
}
