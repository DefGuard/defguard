use std::sync::{
    Arc, LazyLock,
    atomic::{AtomicBool, Ordering},
};

use axum::{
    body::Body,
    http::{HeaderName, HeaderValue, Request, header},
    middleware::Next,
    response::Response,
};
use defguard_common::db::{
    Id,
    models::{DeviceLoginEvent, User},
};
use sqlx::PgPool;
use ua_parser::{Extractor, Regexes, device, os, user_agent};

use crate::mail::templates::{SessionContext, TemplateError, new_device_login_mail};

// Header name constants not yet present in the `http` crate v1.x standard set.
const PERMISSIONS_POLICY: HeaderName = HeaderName::from_static("permissions-policy");
const CROSS_ORIGIN_OPENER_POLICY: HeaderName =
    HeaderName::from_static("cross-origin-opener-policy");
const CROSS_ORIGIN_RESOURCE_POLICY: HeaderName =
    HeaderName::from_static("cross-origin-resource-policy");

/// Injects baseline security response headers on every response.
pub(crate) async fn security_headers_middleware(
    tls_active: Arc<AtomicBool>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let is_api = request.uri().path().starts_with("/api/");
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // `X-Content-Type-Options: nosniff` - prevents MIME-type sniffing/confusion attacks
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );

    // `Referrer-Policy: strict-origin-when-cross-origin` - avoids leaking internal URLs via Referer to external sites
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // `Permissions-Policy: geolocation=(), camera=(), microphone=()` - disables unused browser APIs
    headers.insert(
        PERMISSIONS_POLICY,
        HeaderValue::from_static("geolocation=(), camera=(), microphone=()"),
    );

    // `Cross-Origin-Opener-Policy: same-origin` - severs window.opener references, preventing reverse tabnapping
    headers.insert(
        CROSS_ORIGIN_OPENER_POLICY,
        HeaderValue::from_static("same-origin"),
    );

    // `Cross-Origin-Resource-Policy: same-origin` - blocks cross-origin embedding of application resources
    headers.insert(
        CROSS_ORIGIN_RESOURCE_POLICY,
        HeaderValue::from_static("same-origin"),
    );

    // `X-Frame-Options: DENY` - clickjacking defense for browsers without CSP frame-ancestors support
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));

    // `Content-Security-Policy: frame-ancestors 'none'` - prevents framing/clickjacking
    // Use entry/or_insert so individual handlers can override CSP (e.g. per-request nonces)
    headers
        .entry(header::CONTENT_SECURITY_POLICY)
        .or_insert(HeaderValue::from_static("frame-ancestors 'none';"));

    // `Strict-Transport-Security` - only sent over TLS; ignored and potentially harmful over plain HTTP (RFC 6797 §7.2)
    let tls = tls_active.load(Ordering::Relaxed);
    if tls {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000"),
        );
    }

    // `Cache-Control: no-store` - prevents browsers and caches from storing sensitive API responses
    if is_api {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }

    response
}

/// Family reported by uap-core when nothing matches.
const UNKNOWN_FAMILY: &str = "Other";

pub static USER_AGENT_PARSER: LazyLock<Extractor<'static>> = LazyLock::new(|| {
    let regexes: Regexes =
        serde_yaml::from_slice(include_bytes!("../user_agent_header_regexes.yaml"))
            .expect("Failed to load user agent regexes");
    Extractor::try_from(regexes).expect("Parser creation failed")
});

/// User agent split into browser, OS and device by [`USER_AGENT_PARSER`].
pub(crate) struct ParsedUserAgent<'a> {
    browser: Option<user_agent::ValueRef<'a>>,
    os: Option<os::ValueRef<'a>>,
    device: Option<device::ValueRef<'a>>,
}

impl<'a> ParsedUserAgent<'a> {
    #[must_use]
    pub(crate) fn parse(user_agent: &'a str) -> Self {
        let (browser, os, device) = USER_AGENT_PARSER.extract(user_agent);
        Self {
            browser,
            os,
            device,
        }
    }

    fn browser_family(&self) -> &str {
        self.browser
            .as_ref()
            .map_or(UNKNOWN_FAMILY, |browser| &browser.family)
    }

    fn os_family(&self) -> &str {
        self.os.as_ref().map_or(UNKNOWN_FAMILY, |os| &os.os)
    }

    fn device_family(&self) -> &str {
        self.device
            .as_ref()
            .map_or(UNKNOWN_FAMILY, |device| &device.device)
    }

    fn device_model(&self) -> Option<&str> {
        self.device.as_ref()?.model.as_deref()
    }

    fn device_brand(&self) -> Option<&str> {
        self.device.as_ref()?.brand.as_deref()
    }

    /// OS version as "major[.minor[.patch]]", or empty if unknown.
    fn os_version(&self) -> String {
        let Some(os) = &self.os else {
            return String::new();
        };
        [&os.major, &os.minor, &os.patch]
            .into_iter()
            .map_while(|part| part.as_deref())
            .collect::<Vec<_>>()
            .join(".")
    }
}

#[must_use]
pub fn get_device_info(user_agent: &str) -> String {
    let escaped = tera::escape_html(user_agent);
    get_user_agent_device(&ParsedUserAgent::parse(&escaped))
}

#[must_use]
pub(crate) fn get_user_agent_device(user_agent: &ParsedUserAgent) -> String {
    format!(
        "{}, OS: {} {}, {}",
        user_agent.device_model().unwrap_or("unknown model"),
        user_agent.os_family(),
        user_agent.os_version(),
        user_agent.browser_family(),
    )
}

fn get_user_agent_device_login_data(
    user_id: Id,
    ip_address: String,
    event_type: String,
    user_agent: &ParsedUserAgent,
) -> DeviceLoginEvent {
    DeviceLoginEvent::new(
        user_id,
        ip_address,
        user_agent.device_model().map(ToString::to_string),
        user_agent.device_family().to_string(),
        user_agent.device_brand().map(ToString::to_string),
        user_agent.os_family().to_string(),
        user_agent.browser_family().to_string(),
        event_type,
    )
}

pub(crate) async fn check_new_device_login(
    pool: &PgPool,
    session: &SessionContext,
    user: &User<Id>,
    ip_address: String,
    event_type: String,
    agent: ParsedUserAgent<'_>,
) -> Result<(), TemplateError> {
    let device_login_event =
        get_user_agent_device_login_data(user.id, ip_address, event_type, &agent);

    if let Ok(Some(created_device_login_event)) = device_login_event
        .check_if_device_already_logged_in(pool)
        .await
    {
        let mut conn = pool.begin().await?;
        new_device_login_mail(
            &user.email,
            &mut conn,
            Some(session),
            created_device_login_event.created,
        )
        .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_user_agent() {
        let agent = ParsedUserAgent::parse(
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_6_1 like Mac OS X) AppleWebKit/605.1.15 \
            (KHTML, like Gecko) Version/17.6 Mobile/15E148 Safari/604.1",
        );
        assert_eq!(agent.device_model(), Some("iPhone"));
        assert_eq!(agent.device_brand(), Some("Apple"));
        assert_eq!(agent.device_family(), "iPhone");
        assert_eq!(agent.os_family(), "iOS");
        assert_eq!(agent.os_version(), "17.6.1");
        assert_eq!(agent.browser_family(), "Mobile Safari");
        assert_eq!(
            get_user_agent_device(&agent),
            "iPhone, OS: iOS 17.6.1, Mobile Safari"
        );

        let agent = ParsedUserAgent::parse("");
        assert_eq!(agent.device_model(), None);
        assert_eq!(agent.device_family(), "Other");
        assert_eq!(
            get_user_agent_device(&agent),
            "unknown model, OS: Other , Other"
        );
    }
}
