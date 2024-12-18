pub mod db;
pub mod directory_sync;
pub mod grpc;
pub mod handlers;
pub mod license;
pub mod limits;
use license::{get_cached_license, validate_license};
use limits::get_counts;

pub(crate) fn needs_enterprise_license() -> bool {
    get_counts().is_over_limit()
}

pub(crate) fn is_enterprise_enabled() -> bool {
    debug!("Checking if enterprise is enabled");
    match needs_enterprise_license() {
        true => {
            debug!("User is over limit, checking his license");
            let license = get_cached_license();
            let validation_result = validate_license(license.as_ref());
            debug!("License validation result: {:?}", validation_result);
            validation_result.is_ok()
        }
        false => {
            debug!("User is not over limit, allowing enterprise features");
            true
        }
    }
}
