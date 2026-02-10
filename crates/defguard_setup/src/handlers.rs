use std::sync::{Arc, Mutex};

use axum::{Extension, Json};
use axum_client_ip::InsecureClientIp;
use axum_extra::{
    TypedHeader,
    extract::{
        CookieJar,
        cookie::{Cookie, SameSite},
    },
    headers::UserAgent,
};
use defguard_certs::{der_to_pem, parse_certificate_info, parse_pem_certificate};
use defguard_common::db::models::{
    Session, SessionState, Settings, User,
    group::Group,
    settings::{InitialSetupStep, update_current_settings},
};
use defguard_core::{
    auth::{
        AdminOrSetupRole, SessionInfo,
        failed_login::{FailedLoginMap, check_failed_logins, log_failed_login_attempt},
    },
    error::WebError,
    handlers::{ApiResponse, ApiResult, SESSION_COOKIE_NAME},
    headers::get_device_info,
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::oneshot;
use tracing::{debug, info};

async fn advance_setup_to_step(pool: &PgPool, step: InitialSetupStep) -> Result<(), WebError> {
    let mut settings = Settings::get_current_settings();

    // Don't try to advance if setup is already completed
    if settings.initial_setup_completed {
        debug!("Not advancing setup step as initial setup is already completed");
        return Ok(());
    }

    if settings.initial_setup_step < step {
        settings.initial_setup_step = step;
        update_current_settings(pool, settings).await?;
        info!("Advanced initial wizard setup to step {:?}", step);
    } else {
        debug!(
            "Not advancing initial wizard setup step from {:?} to {:?} as it is not a forward step",
            settings.initial_setup_step, step
        );
    }
    Ok(())
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CreateAdmin {
    first_name: String,
    last_name: String,
    username: String,
    email: String,
    password: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SetupLogin {
    username: String,
    password: String,
}

pub async fn create_admin(
    cookies: CookieJar,
    user_agent: TypedHeader<UserAgent>,
    InsecureClientIp(insecure_ip): InsecureClientIp,
    Extension(pool): Extension<PgPool>,
    Json(admin): Json<CreateAdmin>,
) -> Result<(CookieJar, ApiResponse), WebError> {
    advance_setup_to_step(&pool, InitialSetupStep::AdminUser).await?;
    info!(
        "Creating initial admin user {} ({})",
        admin.username, admin.email
    );
    let user = User::new(
        admin.username,
        Some(admin.password.as_str()),
        admin.last_name,
        admin.first_name,
        admin.email,
        None,
    )
    .save(&pool)
    .await?;

    debug!("Initial admin user created with ID {}", user.id);
    let mut settings = Settings::get_current_settings();
    settings.default_admin_id = Some(user.id);
    update_current_settings(&pool, settings).await?;
    debug!("Initial admin user set as default admin in settings");

    let device_info = get_device_info(user_agent.as_str());

    Session::delete_expired(&pool).await?;
    let session = Session::new(
        user.id,
        SessionState::PasswordVerified,
        insecure_ip.to_string(),
        Some(device_info),
    );
    session.save(&pool).await?;

    let auth_cookie = Cookie::build((SESSION_COOKIE_NAME, session.id.clone()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax);
    let cookies = cookies.add(auth_cookie);

    info!("Initial admin user created");

    advance_setup_to_step(&pool, InitialSetupStep::GeneralConfiguration).await?;

    Ok((cookies, ApiResponse::with_status(StatusCode::CREATED)))
}

pub async fn setup_login(
    cookies: CookieJar,
    user_agent: TypedHeader<UserAgent>,
    InsecureClientIp(insecure_ip): InsecureClientIp,
    Extension(pool): Extension<PgPool>,
    Extension(failed_logins): Extension<Arc<Mutex<FailedLoginMap>>>,
    Json(login): Json<SetupLogin>,
) -> Result<(CookieJar, ApiResponse), WebError> {
    let settings = Settings::get_current_settings();
    if settings.initial_setup_completed {
        return Err(WebError::Forbidden(
            "Initial setup already completed".to_string(),
        ));
    }
    let default_admin_id = settings
        .default_admin_id
        .ok_or_else(|| WebError::Forbidden("Default admin user not set".into()))?;

    check_failed_logins(&failed_logins, &login.username)?;

    let mut conn = pool.acquire().await?;
    let user = match User::find_by_username_or_email(&mut conn, &login.username).await? {
        Some(user) => user,
        None => {
            log_failed_login_attempt(&failed_logins, &login.username);
            return Err(WebError::Authentication);
        }
    };

    if user.verify_password(&login.password).is_err() {
        log_failed_login_attempt(&failed_logins, &login.username);
        return Err(WebError::Authentication);
    }

    if !user.is_active {
        return Err(WebError::Authentication);
    }

    if user.id != default_admin_id {
        return Err(WebError::Forbidden("access denied".into()));
    }

    let device_info = get_device_info(user_agent.as_str());

    Session::delete_expired(&pool).await?;
    let session = Session::new(
        user.id,
        SessionState::PasswordVerified,
        insecure_ip.to_string(),
        Some(device_info),
    );
    session.save(&pool).await?;

    let auth_cookie = Cookie::build((SESSION_COOKIE_NAME, session.id.clone()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax);
    let cookies = cookies.add(auth_cookie);

    Ok((cookies, ApiResponse::with_status(StatusCode::OK)))
}

pub async fn setup_session(session: SessionInfo) -> ApiResult {
    let settings = Settings::get_current_settings();
    if settings.initial_setup_completed {
        return Err(WebError::Forbidden(
            "Initial setup already completed".to_string(),
        ));
    }
    let default_admin_id = settings
        .default_admin_id
        .ok_or_else(|| WebError::Forbidden("Default admin user not set".into()))?;
    if session.user.id != default_admin_id {
        return Err(WebError::Forbidden("access denied".into()));
    }
    Ok(ApiResponse::with_status(StatusCode::OK))
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GeneralConfig {
    defguard_url: String,
    default_admin_group_name: String,
    default_authentication: u32,
    default_mfa_code_lifetime: u32,
    public_proxy_url: String,
}

pub async fn set_general_config(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
    Json(general_config): Json<GeneralConfig>,
) -> ApiResult {
    info!("Applying initial general configuration settings");
    debug!(
        "General configuration received: defguard_url={}, default_admin_group_name={}, default_authentication={}, default_mfa_code_lifetime={}",
        general_config.defguard_url,
        general_config.default_admin_group_name,
        general_config.default_authentication,
        general_config.default_mfa_code_lifetime,
    );
    let default_admin_group_name = general_config.default_admin_group_name.clone();
    let mut settings = Settings::get_current_settings();
    settings.defguard_url = general_config.defguard_url;
    settings.default_admin_group_name = general_config.default_admin_group_name;
    settings.authentication_period_days = general_config
        .default_authentication
        .try_into()
        .map_err(|err| {
            WebError::BadRequest(format!("Invalid authentication period days: {err}"))
        })?;
    settings.mfa_code_timeout_seconds = general_config
        .default_mfa_code_lifetime
        .try_into()
        .map_err(|err| WebError::BadRequest(format!("Invalid MFA code timeout seconds: {err}")))?;
    settings.public_proxy_url = general_config.public_proxy_url;
    update_current_settings(&pool, settings).await?;
    let settings = Settings::get_current_settings();
    debug!("Settings persisted");

    let admin_group =
        if let Some(mut group) = Group::find_by_name(&pool, &default_admin_group_name).await? {
            debug!(
                "Admin group {} found, marking as admin",
                default_admin_group_name
            );
            group.is_admin = true;
            group.save(&pool).await?;
            group
        } else {
            debug!(
                "Admin group {} not found, creating",
                default_admin_group_name
            );
            let mut group = Group::new(&default_admin_group_name);
            group.is_admin = true;
            group.save(&pool).await?
        };

    let admin_id = settings
        .default_admin_id
        .ok_or_else(|| WebError::DbError("Default admin user ID not set in settings".into()))?;

    let admin_user = User::find_by_id(&pool, admin_id).await?.ok_or_else(|| {
        WebError::ObjectNotFound(format!("Admin user with ID '{admin_id}' not found"))
    })?;
    debug!(
        "Assigning admin user {} to admin group {}",
        admin_user.username, admin_group.name
    );
    admin_user.add_to_group(&pool, &admin_group).await?;

    info!("Initial general configuration applied");

    advance_setup_to_step(&pool, InitialSetupStep::Ca).await?;

    Ok(ApiResponse::with_status(StatusCode::CREATED))
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CreateCA {
    common_name: String,
    email: String,
    validity_period_years: u32,
}

pub async fn create_ca(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
    Json(ca_info): Json<CreateCA>,
) -> ApiResult {
    info!("Creating new certificate authority");
    debug!(
        "CA request details: common_name={}, email={}, validity_period_years={}",
        ca_info.common_name, ca_info.email, ca_info.validity_period_years
    );
    let mut settings = Settings::get_current_settings();
    let ca = defguard_certs::CertificateAuthority::new(
        &ca_info.common_name,
        &ca_info.email,
        ca_info.validity_period_years * 365,
    )?;

    let (cert_der, key_der) = (ca.cert_der().to_vec(), ca.key_pair_der().to_vec());

    settings.ca_cert_der = Some(cert_der);
    settings.ca_key_der = Some(key_der);
    settings.ca_expiry = Some(ca.expiry()?);

    update_current_settings(&pool, settings).await?;

    info!("Certificate authority created and stored");

    advance_setup_to_step(&pool, InitialSetupStep::CaSummary).await?;

    Ok(ApiResponse::with_status(StatusCode::CREATED))
}

pub async fn get_ca(_: AdminOrSetupRole, Extension(pool): Extension<PgPool>) -> ApiResult {
    debug!("Fetching certificate authority details");
    let settings = Settings::get_current_settings();
    if let Some(ca_cert_der) = settings.ca_cert_der {
        let ca_pem = der_to_pem(&ca_cert_der, defguard_certs::PemLabel::Certificate)?;
        let info = parse_certificate_info(&ca_cert_der)?;
        let valid_for_days = (info.not_after.and_utc() - chrono::Utc::now()).num_days();

        debug!(
            "Certificate authority details prepared: subject_common_name={}, valid_for_days={}",
            info.subject_common_name, valid_for_days
        );

        advance_setup_to_step(&pool, InitialSetupStep::EdgeComponent).await?;

        Ok(ApiResponse::new(
            json!({ "ca_cert_pem": ca_pem, "subject_common_name": info.subject_common_name, "not_before": info.not_before, "not_after": info.not_after, "valid_for_days": valid_for_days }),
            StatusCode::OK,
        ))
    } else {
        Err(WebError::ObjectNotFound(
            "CA certificate not found".to_string(),
        ))
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UploadCA {
    cert_file: String,
}

pub async fn upload_ca(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
    Json(ca_info): Json<UploadCA>,
) -> ApiResult {
    info!("Uploading existing certificate authority");
    let cert_der = parse_pem_certificate(&ca_info.cert_file)?;
    let expiry = parse_certificate_info(&cert_der)?.not_after;

    let mut settings = Settings::get_current_settings();
    settings.ca_cert_der = Some(cert_der.to_vec());
    settings.ca_key_der = None; // Key is not provided when uploading CA
    settings.ca_expiry = Some(expiry);

    update_current_settings(&pool, settings).await?;

    advance_setup_to_step(&pool, InitialSetupStep::CaSummary).await?;

    info!("Certificate authority uploaded and stored");

    Ok(ApiResponse::with_status(StatusCode::CREATED))
}

pub async fn finish_setup(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
    Extension(setup_shutdown_tx): Extension<Arc<Mutex<Option<oneshot::Sender<()>>>>>,
) -> ApiResult {
    info!("Finishing initial setup");
    let mut settings = Settings::get_current_settings();
    settings.initial_setup_step = InitialSetupStep::Finished;
    settings.initial_setup_completed = true;
    update_current_settings(&pool, settings).await?;
    if let Some(tx) = setup_shutdown_tx
        .lock()
        .expect("Failed to lock setup shutdown sender")
        .take()
    {
        let _ = tx.send(());
        info!("Initial setup completed and shutdown signal sent");
    } else {
        return Err(WebError::BadRequest(
            "Setup shutdown sender no longer available".to_string(),
        ));
    }

    Ok(ApiResponse::with_status(StatusCode::OK))
}
