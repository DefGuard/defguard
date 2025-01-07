pub mod db;
pub mod directory_sync;
pub mod grpc;
pub mod handlers;
pub mod license;
pub mod limits;
use license::{get_cached_license, validate_license};
use limits::get_counts;

pub(crate) fn is_enterprise_enabled() -> bool {
    debug!("Checking if enterprise features should be enabled");
    let counts = get_counts();
    if counts.needs_enterprise_license() {
        debug!("User is over limit, checking his license");
        let license = get_cached_license();
        let validation_result = validate_license(license.as_ref(), &counts);
        debug!("License validation result: {:?}", validation_result);
        validation_result.is_ok()
    } else {
        debug!("User is not over limit, allowing enterprise features");
        true
    }
}
