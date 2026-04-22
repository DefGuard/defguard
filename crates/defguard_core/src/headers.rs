use std::{
    borrow::Borrow,
    sync::{LazyLock, atomic::Ordering},
};

use axum::{
    body::Body,
    extract::State,
    http::{HeaderName, HeaderValue, Request, header},
    middleware::Next,
    response::Response,
};

use crate::appstate::AppState;
use defguard_common::db::{
    Id,
    models::{DeviceLoginEvent, User},
};
use defguard_mail::templates::{SessionContext, TemplateError, new_device_login_mail};
use sqlx::PgPool;
use uaparser::{Client, Parser, UserAgentParser};

// Header name constants not yet present in the `http` crate v1.x standard set.
const PERMISSIONS_POLICY: HeaderName = HeaderName::from_static("permissions-policy");
const CROSS_ORIGIN_OPENER_POLICY: HeaderName =
    HeaderName::from_static("cross-origin-opener-policy");
const CROSS_ORIGIN_RESOURCE_POLICY: HeaderName =
    HeaderName::from_static("cross-origin-resource-policy");

/// Injects baseline security response headers on every response.
pub(crate) async fn security_headers_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
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
    let tls = state.tls_active.load(Ordering::Relaxed);
    if tls {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }
    response
}

pub static USER_AGENT_PARSER: LazyLock<UserAgentParser> = LazyLock::new(|| {
    let regexes = include_bytes!("../user_agent_header_regexes.yaml");
    UserAgentParser::from_bytes(regexes).expect("Parser creation failed")
});

#[must_use]
pub fn get_device_info(user_agent: &str) -> String {
    let escaped = tera::escape_html(user_agent);
    let client = USER_AGENT_PARSER.parse(&escaped);
    get_user_agent_device(&client)
}

#[must_use]
pub(crate) fn get_user_agent_device(user_agent_client: &Client) -> String {
    let device_type = user_agent_client
        .device
        .model
        .as_ref()
        .map_or("unknown model", Borrow::borrow);

    let mut device_version = String::new();
    if let Some(major) = &user_agent_client.os.major {
        device_version.push_str(major);

        if let Some(minor) = &user_agent_client.os.minor {
            device_version.push('.');
            device_version.push_str(minor);

            if let Some(patch) = &user_agent_client.os.patch {
                device_version.push('.');
                device_version.push_str(patch);
            }
        }
    }

    let mut device_os = user_agent_client.os.family.to_string();
    device_os.push(' ');
    device_os.push_str(&device_version);
    device_os.push_str(", ");
    device_os.push_str(&user_agent_client.user_agent.family);

    format!("{device_type}, OS: {device_os}")
}

fn get_user_agent_device_login_data(
    user_id: Id,
    ip_address: String,
    event_type: String,
    user_agent_client: &Client,
) -> DeviceLoginEvent {
    let model = user_agent_client
        .device
        .model
        .as_ref()
        .map(ToString::to_string);

    let brand = user_agent_client
        .device
        .brand
        .as_ref()
        .map(ToString::to_string);

    let family = user_agent_client.device.family.to_string();
    let os_family = user_agent_client.os.family.to_string();
    let browser = user_agent_client.user_agent.family.to_string();

    DeviceLoginEvent::new(
        user_id, ip_address, model, family, brand, os_family, browser, event_type,
    )
}

pub(crate) async fn check_new_device_login(
    pool: &PgPool,
    session: &SessionContext,
    user: &User<Id>,
    ip_address: String,
    event_type: String,
    agent: Client<'_>,
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
