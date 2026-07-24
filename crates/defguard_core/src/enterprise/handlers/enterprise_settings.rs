use std::collections::HashSet;

use axum::{Json, extract::State, http::StatusCode};
use defguard_common::{db::Id, types::proxy::ProxyControlMessage};
use sqlx::{PgConnection, PgPool, query_scalar};
use struct_patch::Patch;

use super::LicenseInfo;
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::{
        db::models::{
            enterprise_settings::{
                ClientTrafficPolicy, EnterpriseSettings, EnterpriseSettingsInfo,
                EnterpriseSettingsPatch, GroupClientTrafficPolicies,
            },
            group_client_traffic_policy::GroupClientTrafficPolicy,
        },
        is_business_license_active,
    },
    error::WebError,
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    handlers::{ApiResponse, ApiResult},
};

#[derive(Deserialize)]
/// Request payload for partially updating enterprise settings and group policies.
pub struct EnterpriseSettingsPatchRequest {
    #[serde(flatten)]
    pub settings: EnterpriseSettingsPatch,
    pub group_client_traffic_policies: Option<GroupClientTrafficPolicies>,
}

fn policy_assignments(policies: &GroupClientTrafficPolicies) -> Vec<(Id, ClientTrafficPolicy)> {
    policies
        .none
        .iter()
        .map(|&id| (id, ClientTrafficPolicy::None))
        .chain(
            policies
                .disable_all_traffic
                .iter()
                .map(|&id| (id, ClientTrafficPolicy::DisableAllTraffic)),
        )
        .chain(
            policies
                .force_all_traffic
                .iter()
                .map(|&id| (id, ClientTrafficPolicy::ForceAllTraffic)),
        )
        .collect()
}

async fn validate_policy_assignments(
    transaction: &mut PgConnection,
    policies: &GroupClientTrafficPolicies,
) -> Result<(), WebError> {
    let assignments = policy_assignments(policies);
    let mut group_ids = HashSet::with_capacity(assignments.len());
    for (group_id, _) in &assignments {
        if !group_ids.insert(*group_id) {
            return Err(WebError::BadRequest(
                "A group cannot be assigned to multiple client traffic policies.".into(),
            ));
        }
    }

    let group_ids = group_ids.into_iter().collect::<Vec<_>>();
    if group_ids.is_empty() {
        return Ok(());
    }
    let existing_ids: HashSet<Id> =
        query_scalar!("SELECT id FROM \"group\" WHERE id = ANY($1)", &group_ids)
            .fetch_all(&mut *transaction)
            .await?
            .into_iter()
            .collect();
    if existing_ids.len() != group_ids.len() {
        return Err(WebError::BadRequest(
            "One or more client traffic policy groups do not exist.".into(),
        ));
    }
    Ok(())
}

async fn settings_info(
    pool: &PgPool,
    settings: EnterpriseSettings,
) -> Result<EnterpriseSettingsInfo, sqlx::Error> {
    let group_policies = if is_business_license_active() {
        GroupClientTrafficPolicy::grouped(GroupClientTrafficPolicy::all(pool).await?)
    } else {
        GroupClientTrafficPolicies::default()
    };
    Ok(EnterpriseSettingsInfo::new(settings, group_policies))
}

pub async fn get_enterprise_settings(
    session: SessionInfo,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!(
        "User {} retrieving enterprise settings",
        session.user.username
    );
    let settings = EnterpriseSettings::get(&appstate.pool).await?;
    debug!(
        "User {} retrieved enterprise settings",
        session.user.username
    );
    Ok(ApiResponse::json(
        settings_info(&appstate.pool, settings).await?,
        StatusCode::OK,
    ))
}

pub async fn patch_enterprise_settings(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Json(data): Json<EnterpriseSettingsPatchRequest>,
) -> ApiResult {
    debug!(
        "Admin {} patching enterprise settings.",
        session.user.username,
    );
    let mut transaction = appstate.pool.begin().await?;
    let mut settings = EnterpriseSettings::get(&mut *transaction).await?;
    let old_group_policies =
        GroupClientTrafficPolicy::grouped(GroupClientTrafficPolicy::all(&mut *transaction).await?);

    // snapshot for audit event
    let old_settings = settings.clone();
    // snapshot values for broadcast comparison
    let old_display_password_reset = old_settings.display_password_reset;
    let old_display_download_step = old_settings.display_download_step;

    settings.apply(data.settings);
    let group_policies = if let Some(group_policies) = data.group_client_traffic_policies {
        validate_policy_assignments(&mut transaction, &group_policies).await?;
        GroupClientTrafficPolicy::replace_all(&mut transaction, &group_policies).await?;
        group_policies
    } else {
        old_group_policies.clone()
    };
    settings.save(&mut *transaction).await?;
    transaction.commit().await?;

    let before = EnterpriseSettingsInfo::new(old_settings, old_group_policies);
    let after = EnterpriseSettingsInfo::new(settings.clone(), group_policies);
    info!(
        "Admin {} patched enterprise settings.",
        session.user.username
    );

    appstate.emit_event(ApiEvent {
        context: ApiRequestContext::new(
            session.user.id,
            session.user.username.clone(),
            None::<std::net::IpAddr>,
            "web".into(),
        ),
        event: Box::new(ApiEventType::EnterpriseSettingsUpdated { before, after }),
    })?;

    // Broadcast updated public settings to proxies only if they changed.
    if (settings.display_password_reset != old_display_password_reset
        || settings.display_download_step != old_display_download_step)
        && let Err(err) = appstate
            .proxy_control_tx
            .send(ProxyControlMessage::BroadcastPublicSettings {
                display_password_reset: settings.edge_can_display_password_reset(),
                display_download_step: settings.display_download_step,
            })
            .await
    {
        error!("Failed to broadcast PublicSettings to proxies: {err:?}");
    }

    Ok(ApiResponse::default())
}
