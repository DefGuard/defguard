use axum::http::HeaderMap;
use defguard_common::config::server_config;

const X_FORWARDED_PROTO: &str = "x-forwarded-proto";

pub(super) fn setup_cookie_secure(headers: &HeaderMap) -> bool {
    resolve_setup_cookie_secure(headers, server_config().cookie_insecure)
}

fn resolve_setup_cookie_secure(headers: &HeaderMap, cookie_insecure: Option<bool>) -> bool {
    cookie_insecure.map_or(
        headers
            .get(X_FORWARDED_PROTO)
            .and_then(|header| header.to_str().ok())
            .is_some_and(|protocol| protocol == "https"),
        |insecure| !insecure,
    )
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};

    use super::{X_FORWARDED_PROTO, resolve_setup_cookie_secure};

    fn headers(protocol: Option<&'static str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(protocol) = protocol {
            headers.insert(X_FORWARDED_PROTO, HeaderValue::from_static(protocol));
        }
        headers
    }

    #[test]
    fn explicit_cookie_setting_is_inverted() {
        assert!(resolve_setup_cookie_secure(&headers(None), Some(false)));
        assert!(!resolve_setup_cookie_secure(&headers(None), Some(true)));
    }

    #[test]
    fn forwarded_https_enables_secure_cookie() {
        assert!(resolve_setup_cookie_secure(&headers(Some("https")), None));
    }

    #[test]
    fn missing_forwarded_proto_defaults_to_insecure() {
        assert!(!resolve_setup_cookie_secure(&headers(None), None));
    }

    #[test]
    fn explicit_cookie_setting_beats_forwarded_proto() {
        assert!(!resolve_setup_cookie_secure(
            &headers(Some("https")),
            Some(true)
        ));
        assert!(resolve_setup_cookie_secure(
            &headers(Some("http")),
            Some(false)
        ));
    }
}
