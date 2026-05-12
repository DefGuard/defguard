pub mod auth;
pub mod config;
pub mod csv;
pub mod db;
pub mod globals;
pub mod hex;
pub mod messages;
pub mod random;
pub mod secret;
pub mod types;
pub mod utils;

pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "+", env!("VERGEN_GIT_SHA"));
pub const CARGO_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Version reported to external systems.
///
/// Release workflows can override this with `DEFGUARD_BUILD_VERSION`.
pub const REPORTED_VERSION: &str = resolve_reported_version(
    option_env!("DEFGUARD_BUILD_VERSION"),
    env!("CARGO_PKG_VERSION"),
);

const fn resolve_reported_version(
    build_version: Option<&'static str>,
    cargo_version: &'static str,
) -> &'static str {
    match build_version {
        Some(version) if !version.is_empty() => version,
        _ => cargo_version,
    }
}

// WireGuard key length in bytes.
pub const KEY_LENGTH: usize = 32;

#[cfg(test)]
mod tests {
    use super::resolve_reported_version;

    #[test]
    fn reported_version_uses_build_override_for_prereleases_and_falls_back_otherwise() {
        assert_eq!(
            resolve_reported_version(Some("2.0.0-beta.1"), "2.0.0"),
            "2.0.0-beta.1"
        );
        assert_eq!(resolve_reported_version(Some(""), "2.0.0"), "2.0.0");
        assert_eq!(resolve_reported_version(None, "2.0.0"), "2.0.0");
    }
}
