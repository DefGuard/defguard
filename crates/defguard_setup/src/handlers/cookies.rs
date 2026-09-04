use axum::http::HeaderMap;
use defguard_common::config::server_config;

const X_FORWARDED_PROTO: &str = "x-forwarded-proto";

pub(super) fn setup_cookie_secure(headers: &HeaderMap) -> bool {
    setup_cookie_secure_with(headers, server_config().cookie_insecure)
}

/// Resolves the setup cookie's `Secure` flag using an injected configuration override.
///
/// The override takes precedence over the browser-facing scheme from `X-Forwarded-Proto`.
fn setup_cookie_secure_with(headers: &HeaderMap, cookie_insecure: Option<bool>) -> bool {
    cookie_insecure.map_or(
        headers
            .get_all(X_FORWARDED_PROTO)
            .iter()
            .next_back()
            .and_then(|header| header.to_str().ok())
            .and_then(|value| value.split(',').next_back())
            .is_some_and(|protocol| protocol.trim().eq_ignore_ascii_case("https")),
        |insecure| !insecure,
    )
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};

    use super::{X_FORWARDED_PROTO, setup_cookie_secure_with};

    fn headers(protocol: Option<&'static str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(protocol) = protocol {
            headers.insert(X_FORWARDED_PROTO, HeaderValue::from_static(protocol));
        }
        headers
    }

    #[test]
    fn test_explicit_cookie_setting_is_inverted() {
        assert!(setup_cookie_secure_with(&headers(None), Some(false)));
        assert!(!setup_cookie_secure_with(&headers(None), Some(true)));
    }

    #[test]
    fn test_forwarded_https_enables_secure_cookie() {
        assert!(setup_cookie_secure_with(&headers(Some("https")), None));
    }

    #[test]
    fn test_uppercase_forwarded_https_enables_secure_cookie() {
        assert!(setup_cookie_secure_with(&headers(Some("HTTPS")), None));
    }

    #[test]
    fn test_comma_separated_forwarded_https_enables_secure_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(X_FORWARDED_PROTO, HeaderValue::from_static("http, https"));

        assert!(setup_cookie_secure_with(&headers, None));
    }

    #[test]
    fn test_final_forwarded_proto_header_enables_secure_cookie() {
        let mut headers = HeaderMap::new();
        headers.append(X_FORWARDED_PROTO, HeaderValue::from_static("http"));
        headers.append(X_FORWARDED_PROTO, HeaderValue::from_static("HTTPS"));

        assert!(setup_cookie_secure_with(&headers, None));
    }

    #[test]
    fn test_forwarded_http_defaults_to_insecure() {
        assert!(!setup_cookie_secure_with(&headers(Some("http")), None));
    }

    #[test]
    fn test_missing_forwarded_proto_defaults_to_insecure() {
        assert!(!setup_cookie_secure_with(&headers(None), None));
    }

    #[test]
    fn test_explicit_cookie_setting_beats_forwarded_proto() {
        assert!(!setup_cookie_secure_with(
            &headers(Some("https")),
            Some(true)
        ));
        assert!(setup_cookie_secure_with(
            &headers(Some("http")),
            Some(false)
        ));
    }
}
