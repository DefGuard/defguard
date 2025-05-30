pub mod audit_stream;
pub mod db;
pub mod directory_sync;
pub mod firewall;
pub mod grpc;
pub mod handlers;
pub mod ldap;
pub mod license;
pub mod limits;
mod utils;

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

// Is it the free version of the enterprise?
// Free = no valid license + not over the limit
// Paid = valid license or over the limit
pub(crate) fn is_enterprise_free() -> bool {
    debug!("Checking if enterprise features are a part of the free version");
    let counts = get_counts();
    let license = get_cached_license();
    if validate_license(license.as_ref(), &counts).is_ok() {
        false
    } else if counts.needs_enterprise_license() {
        debug!("User is over limit, the enterprise features are not free");
        false
    } else {
        debug!("User is not over limit, the enterprise features are free");
        true
    }
}
