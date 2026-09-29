use std::env;

use base64::{Engine, prelude::BASE64_STANDARD};
use defguard_proto::client_types::ClientPlatformInfo;
use prost::Message;

/// Builds the base64-encoded platform header expected by Proxy.
#[must_use]
pub fn platform_header() -> String {
    let platform = ClientPlatformInfo {
        os_family: env::consts::FAMILY.to_owned(),
        os_type: env::consts::OS.to_owned(),
        version: "load-generator".to_owned(),
        edition: None,
        codename: None,
        bitness: None,
        architecture: Some(env::consts::ARCH.to_owned()),
    };

    BASE64_STANDARD.encode(platform.encode_to_vec())
}
