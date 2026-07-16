use std::collections::HashSet;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use axum_extra::extract::Query as AxumExtraQuery;
use defguard_common::db::{Id, NoId, models::WireguardNetwork};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, Postgres, QueryBuilder};
use utoipa::ToSchema;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::{
        db::models::device_posture::{
            DevicePosture, DevicePostureLocation, DevicePostureOsRule, DevicePostureSnapshot,
            OsType,
        },
        firewall::try_get_location_firewall_config,
        handlers::{DevicePostureFeature, LicenseGated},
        posture::version_list::{
            ANDROID_OS_VERSIONS, DESKTOP_CLIENT_VERSIONS, IOS_OS_VERSIONS, LINUX_KERNEL_VERSIONS,
            MACOS_OS_VERSIONS, MOBILE_CLIENT_VERSIONS, WINDOWS_OS_VERSIONS,
        },
    },
    error::WebError,
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    grpc::GatewayCommand,
    handlers::{
        ApiResponse, ApiResult,
        pagination::{PaginatedApiResponse, PaginatedApiResult, PaginationParams},
    },
    location_management::allowed_peers::get_location_allowed_peers,
};

fn owned_client_versions(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn same_id_set(left: &[Id], right: &[Id]) -> bool {
    left.iter().copied().collect::<HashSet<_>>() == right.iter().copied().collect::<HashSet<_>>()
}

async fn build_location_peer_refresh_commands(
    conn: &mut PgConnection,
    location_ids: impl IntoIterator<Item = Id>,
) -> Result<Vec<GatewayCommand>, WebError> {
    let mut seen = HashSet::new();
    let mut commands = Vec::new();

    for location_id in location_ids {
        if !seen.insert(location_id) {
            continue;
        }

        let Some(location) = WireguardNetwork::find_by_id(&mut *conn, location_id).await? else {
            warn!("Skipping gateway peer refresh for missing location {location_id}");
            continue;
        };

        let peers = get_location_allowed_peers(&location, &mut *conn).await?;
        let maybe_firewall_config = try_get_location_firewall_config(&location, &mut *conn).await?;
        commands.push(GatewayCommand::NetworkModified(
            location.id,
            location,
            peers,
            maybe_firewall_config,
        ));
    }

    Ok(commands)
}

/// Per-OS rule included in a posture check policy API.
///
/// Adding this layer on top of the shared DB type allows us
/// to require different fields for specific platforms.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(tag = "os_type", rename_all = "lowercase")]
pub enum ApiOsRule {
    Windows {
        min_os_version: Option<i32>,
        disk_encryption_required: Option<bool>,
        antivirus_required: Option<bool>,
        ad_domain_joined_required: Option<bool>,
        windows_security_update_max_age: Option<i32>,
    },
    Macos {
        min_os_version: Option<i32>,
        disk_encryption_required: Option<bool>,
        device_integrity_required: Option<bool>,
    },
    Linux {
        min_kernel_version: Option<i32>,
        disk_encryption_required: Option<bool>,
    },
    Ios {
        min_os_version: Option<i32>,
    },
    Android {
        min_os_version: Option<i32>,
        device_integrity_required: Option<bool>,
        android_security_patch_level_max_age: Option<i32>,
    },
}

impl ApiOsRule {
    /// Returns the [`OsType`] discriminant for this rule set.
    fn os_type(&self) -> OsType {
        match self {
            Self::Windows { .. } => OsType::Windows,
            Self::Macos { .. } => OsType::Macos,
            Self::Linux { .. } => OsType::Linux,
            Self::Ios { .. } => OsType::Ios,
            Self::Android { .. } => OsType::Android,
        }
    }

    /// Converts this rule set into an unsaved DB row for the given posture ID.
    #[must_use]
    pub fn into_db_rule(self, posture_id: Id) -> DevicePostureOsRule<NoId> {
        match self {
            Self::Windows {
                min_os_version,
                disk_encryption_required,
                antivirus_required,
                ad_domain_joined_required,
                windows_security_update_max_age,
            } => DevicePostureOsRule {
                id: NoId,
                posture_id,
                os_type: OsType::Windows,
                min_os_version,
                disk_encryption_required,
                antivirus_required,
                ad_domain_joined_required,
                windows_security_update_max_age,
                min_kernel_version: None,
                device_integrity_required: None,
                android_security_patch_level_max_age: None,
            },
            Self::Macos {
                min_os_version,
                disk_encryption_required,
                device_integrity_required,
            } => DevicePostureOsRule {
                id: NoId,
                posture_id,
                os_type: OsType::Macos,
                min_os_version,
                disk_encryption_required,
                antivirus_required: None,
                ad_domain_joined_required: None,
                windows_security_update_max_age: None,
                min_kernel_version: None,
                device_integrity_required,
                android_security_patch_level_max_age: None,
            },
            Self::Linux {
                min_kernel_version,
                disk_encryption_required,
            } => DevicePostureOsRule {
                id: NoId,
                posture_id,
                os_type: OsType::Linux,
                min_os_version: None,
                disk_encryption_required,
                antivirus_required: None,
                ad_domain_joined_required: None,
                windows_security_update_max_age: None,
                min_kernel_version,
                device_integrity_required: None,
                android_security_patch_level_max_age: None,
            },
            Self::Ios { min_os_version } => DevicePostureOsRule {
                id: NoId,
                posture_id,
                os_type: OsType::Ios,
                min_os_version,
                disk_encryption_required: None,
                antivirus_required: None,
                ad_domain_joined_required: None,
                windows_security_update_max_age: None,
                min_kernel_version: None,
                device_integrity_required: None,
                android_security_patch_level_max_age: None,
            },
            Self::Android {
                min_os_version,
                device_integrity_required,
                android_security_patch_level_max_age,
            } => DevicePostureOsRule {
                id: NoId,
                posture_id,
                os_type: OsType::Android,
                min_os_version,
                disk_encryption_required: None,
                antivirus_required: None,
                ad_domain_joined_required: None,
                windows_security_update_max_age: None,
                min_kernel_version: None,
                device_integrity_required,
                android_security_patch_level_max_age,
            },
        }
    }
}

impl From<DevicePostureOsRule<Id>> for ApiOsRule {
    fn from(rule: DevicePostureOsRule<Id>) -> Self {
        match rule.os_type {
            OsType::Windows => Self::Windows {
                min_os_version: rule.min_os_version,
                disk_encryption_required: rule.disk_encryption_required,
                antivirus_required: rule.antivirus_required,
                ad_domain_joined_required: rule.ad_domain_joined_required,
                windows_security_update_max_age: rule.windows_security_update_max_age,
            },
            OsType::Macos => Self::Macos {
                min_os_version: rule.min_os_version,
                disk_encryption_required: rule.disk_encryption_required,
                device_integrity_required: rule.device_integrity_required,
            },
            OsType::Linux => Self::Linux {
                min_kernel_version: rule.min_kernel_version,
                disk_encryption_required: rule.disk_encryption_required,
            },
            OsType::Ios => Self::Ios {
                min_os_version: rule.min_os_version,
            },
            OsType::Android => Self::Android {
                min_os_version: rule.min_os_version,
                device_integrity_required: rule.device_integrity_required,
                android_security_patch_level_max_age: rule.android_security_patch_level_max_age,
            },
        }
    }
}

/// API response type for a device posture check policy.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ApiDevicePosture {
    pub id: Id,
    pub name: String,
    pub description: Option<String>,
    pub min_desktop_client_version: Option<String>,
    pub min_mobile_client_version: Option<String>,
    pub allow_prerelease_client: bool,
    pub os_rules: Vec<ApiOsRule>,
    /// IDs of VPN locations this policy is assigned to.
    pub locations: Vec<Id>,
}

impl From<DevicePosture<Id>> for ApiDevicePosture {
    fn from(p: DevicePosture<Id>) -> Self {
        Self {
            id: p.id,
            name: p.name,
            description: p.description,
            min_desktop_client_version: p.min_desktop_client_version,
            min_mobile_client_version: p.min_mobile_client_version,
            allow_prerelease_client: p.allow_prerelease_client,
            os_rules: Vec::new(),
            locations: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DevicePostureOsVersionCatalog {
    pub windows: Vec<i32>,
    pub macos: Vec<i32>,
    pub ios: Vec<i32>,
    pub android: Vec<i32>,
}

impl DevicePostureOsVersionCatalog {
    #[must_use]
    pub fn new() -> Self {
        Self {
            windows: WINDOWS_OS_VERSIONS.to_vec(),
            macos: MACOS_OS_VERSIONS.to_vec(),
            ios: IOS_OS_VERSIONS.to_vec(),
            android: ANDROID_OS_VERSIONS.to_vec(),
        }
    }
}

impl Default for DevicePostureOsVersionCatalog {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DevicePostureVersionMetadata {
    pub os_versions: DevicePostureOsVersionCatalog,
    pub linux_kernel_versions: Vec<i32>,
    pub desktop_client_versions: Vec<String>,
    pub mobile_client_versions: Vec<String>,
}

impl DevicePostureVersionMetadata {
    #[must_use]
    pub fn new() -> Self {
        Self {
            os_versions: DevicePostureOsVersionCatalog::new(),
            linux_kernel_versions: LINUX_KERNEL_VERSIONS.to_vec(),
            desktop_client_versions: owned_client_versions(DESKTOP_CLIENT_VERSIONS),
            mobile_client_versions: owned_client_versions(MOBILE_CLIENT_VERSIONS),
        }
    }
}

impl Default for DevicePostureVersionMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Request body for creating or updating a device posture check policy.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct EditDevicePosture {
    pub name: String,
    pub description: Option<String>,
    pub min_desktop_client_version: Option<String>,
    pub min_mobile_client_version: Option<String>,
    pub allow_prerelease_client: bool,
    #[serde(default)]
    pub os_rules: Vec<ApiOsRule>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ListDevicePostureFilters {
    #[serde(default)]
    pub windows: Vec<String>,
    #[serde(default)]
    pub macos: Vec<String>,
    #[serde(default)]
    pub linux: Vec<String>,
    #[serde(default)]
    pub ios: Vec<String>,
    #[serde(default)]
    pub android: Vec<String>,
    #[serde(default)]
    pub defguard_desktop: Vec<String>,
    #[serde(default)]
    pub defguard_mobile: Vec<String>,
    #[serde(default)]
    pub defguard: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum OsRequirementFilter {
    DiskEncryption,
    Antivirus,
    AdJoined,
    SecurityUpdates,
    DeviceIntegrity,
}

impl OsRequirementFilter {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "Disk encryption" => Some(Self::DiskEncryption),
            "Antivirus" => Some(Self::Antivirus),
            "AD joined" => Some(Self::AdJoined),
            "Security updates" => Some(Self::SecurityUpdates),
            "Device integrity" => Some(Self::DeviceIntegrity),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum DefguardRequirementFilter {
    PrereleaseAllowed,
}

impl DefguardRequirementFilter {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "Pre-release allowed" => Some(Self::PrereleaseAllowed),
            _ => None,
        }
    }
}

fn append_string_array_filter(
    query_builder: &mut QueryBuilder<Postgres>,
    filters: &[String],
    clause_prefix: &str,
) {
    if filters.is_empty() {
        return;
    }

    query_builder
        .push(clause_prefix)
        .push(" = ANY(")
        .push_bind(filters.to_vec())
        .push(")");
}

fn append_bool_filter(query_builder: &mut QueryBuilder<Postgres>, enabled: bool, clause: &str) {
    if enabled {
        query_builder.push(clause);
    }
}

fn apply_os_rule_filters(
    query_builder: &mut QueryBuilder<Postgres>,
    alias: &str,
    os_type: OsType,
    filters: &[String],
) {
    if filters.is_empty() {
        return;
    }

    let os_type_label = match os_type {
        OsType::Windows => "windows",
        OsType::Macos => "macos",
        OsType::Linux => "linux",
        OsType::Ios => "ios",
        OsType::Android => "android",
    };

    let mut versions = Vec::new();
    let mut requirements = HashSet::new();

    for filter in filters {
        match OsRequirementFilter::parse(filter) {
            Some(requirement) => {
                requirements.insert(requirement);
            }
            None => versions.push(filter.clone()),
        }
    }

    query_builder
        .push(" AND EXISTS (SELECT 1 FROM device_posture_os_rule ")
        .push(alias);
    query_builder
        .push(" WHERE ")
        .push(alias)
        .push(".posture_id = dp.id AND ")
        .push(alias)
        .push(".os_type = '")
        .push(os_type_label)
        .push("'");

    match os_type {
        OsType::Windows | OsType::Macos | OsType::Ios | OsType::Android => {
            append_string_array_filter(
                query_builder,
                &versions,
                &format!(" AND {alias}.min_os_version::text"),
            );
        }
        OsType::Linux => {
            append_string_array_filter(
                query_builder,
                &versions,
                &format!(" AND {alias}.min_kernel_version::text"),
            );
        }
    }

    append_bool_filter(
        query_builder,
        requirements.contains(&OsRequirementFilter::DiskEncryption),
        &format!(" AND COALESCE({alias}.disk_encryption_required, false) = true"),
    );
    append_bool_filter(
        query_builder,
        requirements.contains(&OsRequirementFilter::Antivirus),
        &format!(" AND COALESCE({alias}.antivirus_required, false) = true"),
    );
    append_bool_filter(
        query_builder,
        requirements.contains(&OsRequirementFilter::AdJoined),
        &format!(" AND COALESCE({alias}.ad_domain_joined_required, false) = true"),
    );
    append_bool_filter(
        query_builder,
        requirements.contains(&OsRequirementFilter::SecurityUpdates),
        &format!(" AND {alias}.windows_security_update_max_age IS NOT NULL"),
    );
    append_bool_filter(
        query_builder,
        requirements.contains(&OsRequirementFilter::DeviceIntegrity),
        &format!(" AND COALESCE({alias}.device_integrity_required, false) = true"),
    );

    query_builder.push(")");
}

fn apply_device_posture_filters(
    query_builder: &mut QueryBuilder<Postgres>,
    filters: &ListDevicePostureFilters,
) {
    apply_os_rule_filters(query_builder, "w", OsType::Windows, &filters.windows);
    apply_os_rule_filters(query_builder, "m", OsType::Macos, &filters.macos);
    apply_os_rule_filters(query_builder, "l", OsType::Linux, &filters.linux);
    apply_os_rule_filters(query_builder, "i", OsType::Ios, &filters.ios);
    apply_os_rule_filters(query_builder, "a", OsType::Android, &filters.android);

    append_string_array_filter(
        query_builder,
        &filters.defguard_desktop,
        " AND dp.min_desktop_client_version",
    );
    append_string_array_filter(
        query_builder,
        &filters.defguard_mobile,
        " AND dp.min_mobile_client_version",
    );

    if !filters.defguard.is_empty() {
        let mut requirements = HashSet::new();

        for filter in &filters.defguard {
            if let Some(requirement) = DefguardRequirementFilter::parse(filter) {
                requirements.insert(requirement);
            }
        }

        append_bool_filter(
            query_builder,
            requirements.contains(&DefguardRequirementFilter::PrereleaseAllowed),
            " AND dp.allow_prerelease_client = true",
        );
    }
}

/// Validates the base fields of an [`EditDevicePosture`] request.
///
/// Returns `Err(WebError::BadRequest(...))` if client version fields are set to
/// values not present in their supported version catalogs.
fn validate_device_posture_base(data: &EditDevicePosture) -> Result<(), WebError> {
    if let Some(ref version) = data.min_desktop_client_version
        && !DESKTOP_CLIENT_VERSIONS.contains(&version.as_str())
    {
        return Err(WebError::BadRequest(format!(
            "Unknown desktop client version '{version}'. Valid values: {}",
            DESKTOP_CLIENT_VERSIONS.join(", ")
        )));
    }
    if let Some(ref version) = data.min_mobile_client_version
        && !MOBILE_CLIENT_VERSIONS.contains(&version.as_str())
    {
        return Err(WebError::BadRequest(format!(
            "Unknown mobile client version '{version}'. Valid values: {}",
            MOBILE_CLIENT_VERSIONS.join(", ")
        )));
    }
    validate_device_posture_os_rules(&data.os_rules)
}

fn validate_device_posture_os_rule(rule: &ApiOsRule) -> Result<(), WebError> {
    let os_type = rule.os_type();

    match rule {
        ApiOsRule::Windows {
            min_os_version: Some(v),
            ..
        } if !WINDOWS_OS_VERSIONS.contains(v) => Err(WebError::BadRequest(format!(
            "Unknown min_os_version '{v}' for {os_type:?}. Valid values: {}",
            WINDOWS_OS_VERSIONS
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ))),
        ApiOsRule::Macos {
            min_os_version: Some(v),
            ..
        } if !MACOS_OS_VERSIONS.contains(v) => Err(WebError::BadRequest(format!(
            "Unknown min_os_version '{v}' for {os_type:?}. Valid values: {}",
            MACOS_OS_VERSIONS
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ))),
        ApiOsRule::Ios {
            min_os_version: Some(v),
        } if !IOS_OS_VERSIONS.contains(v) => Err(WebError::BadRequest(format!(
            "Unknown min_os_version '{v}' for {os_type:?}. Valid values: {}",
            IOS_OS_VERSIONS
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ))),
        ApiOsRule::Android {
            min_os_version: Some(v),
            ..
        } if !ANDROID_OS_VERSIONS.contains(v) => Err(WebError::BadRequest(format!(
            "Unknown min_os_version '{v}' for {os_type:?}. Valid values: {}",
            ANDROID_OS_VERSIONS
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ))),
        ApiOsRule::Linux {
            min_kernel_version: Some(kv),
            ..
        } if !LINUX_KERNEL_VERSIONS.contains(kv) => Err(WebError::BadRequest(format!(
            "Unknown min_kernel_version '{kv}'. Valid values: {}",
            LINUX_KERNEL_VERSIONS
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ))),
        _ => Ok(()),
    }
}

/// Validates the `os_rules` list in an [`EditDevicePosture`] request.
///
/// Returns `Err(WebError::BadRequest(...))` if:
/// - the same `os_type` appears more than once, or
/// - `min_os_version` is not in the known list for its OS type, or
/// - `min_kernel_version` is not in the known Linux kernel version list.
fn validate_device_posture_os_rules(os_rules: &[ApiOsRule]) -> Result<(), WebError> {
    let mut seen = HashSet::new();
    for rule in os_rules {
        let os_type = rule.os_type();
        if !seen.insert(std::mem::discriminant(rule)) {
            return Err(WebError::BadRequest(format!(
                "Duplicate os_type '{os_type:?}' in os_rules"
            )));
        }
        validate_device_posture_os_rule(rule)?;
    }
    Ok(())
}

#[utoipa::path(
    post,
    path = "/api/v1/posture",
    tag = "DevicePosture",
    request_body = EditDevicePosture,
    responses(
        (status = 201, description = "Posture check created successfully", body = ApiDevicePosture),
        (status = 400, description = "Bad request - invalid field value"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - enterprise license required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn create_device_posture(
    _license: LicenseGated<DevicePostureFeature>,
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(data): Json<EditDevicePosture>,
) -> ApiResult {
    debug!(
        "User {} creating device posture check {:?}",
        session.user.username, data.name
    );

    validate_device_posture_base(&data)?;

    let EditDevicePosture {
        name,
        description,
        min_desktop_client_version,
        min_mobile_client_version,
        allow_prerelease_client,
        os_rules,
    } = data;

    let mut tx = appstate.pool.begin().await?;

    let posture = DevicePosture {
        id: NoId,
        name,
        description,
        min_desktop_client_version,
        min_mobile_client_version,
        allow_prerelease_client,
    }
    .save(&mut *tx)
    .await?;

    for rule in &os_rules {
        rule.clone().into_db_rule(posture.id).save(&mut *tx).await?;
    }

    tx.commit().await?;

    debug!("Created device posture check {}", posture.id);

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::DevicePostureCreated {
            snapshot: DevicePostureSnapshot {
                device_posture: posture.clone(),
                os_rules: DevicePostureOsRule::find_by_posture(&appstate.pool, posture.id).await?,
                location_ids: Vec::new(),
            },
        }),
    })?;

    let mut response = ApiDevicePosture::from(posture);
    response.os_rules = os_rules;
    Ok(ApiResponse::json(response, StatusCode::CREATED))
}

#[utoipa::path(
    get,
    path = "/api/v1/device-posture/versions",
    tag = "DevicePosture",
    responses(
        (status = 200, description = "Valid device posture OS and client versions", body = DevicePostureVersionMetadata),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
/// Return the backend-owned catalog of selectable posture-check versions.
///
/// # Errors
///
/// Returns an error when the requester is unauthorized or lacks the required license.
pub async fn get_device_posture_versions(_admin: AdminRole, session: SessionInfo) -> ApiResult {
    debug!(
        "User {} fetching device posture version metadata",
        session.user.username
    );

    Ok(ApiResponse::json(
        DevicePostureVersionMetadata::new(),
        StatusCode::OK,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/device-posture",
    tag = "DevicePosture",
    params(
        ("page" = Option<u32>, Query, description = "Page number (default: 1)"),
        ("per_page" = Option<u32>, Query, description = "Items per page (default: 10)"),
    ),
    responses(
        (status = 200, description = "Paginated list of device posture check policies", body = [ApiDevicePosture]),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn list_device_postures(
    _admin: AdminRole,
    session: SessionInfo,
    pagination: Query<PaginationParams>,
    filters: AxumExtraQuery<ListDevicePostureFilters>,
    State(appstate): State<AppState>,
) -> PaginatedApiResult<ApiDevicePosture> {
    let pagination = pagination.0;
    let filters = filters.0;
    debug!(
        "User {} listing device posture checks",
        session.user.username
    );

    let mut conn = appstate.pool.acquire().await?;
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT id, name, description, min_desktop_client_version, \
         min_mobile_client_version, allow_prerelease_client \
         FROM device_posture dp WHERE 1=1 ",
    );
    apply_device_posture_filters(&mut query_builder, &filters);
    query_builder
        .push(" ORDER BY id DESC LIMIT ")
        .push_bind(i64::from(pagination.per_page()))
        .push(" OFFSET ")
        .push_bind(i64::from(pagination.offset()));

    let device_postures = query_builder
        .build_query_as::<DevicePosture<Id>>()
        .fetch_all(&mut *conn)
        .await?;

    let mut count_query_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM device_posture dp WHERE 1=1 ");
    apply_device_posture_filters(&mut count_query_builder, &filters);
    let count: i64 = count_query_builder
        .build_query_scalar()
        .fetch_one(&mut *conn)
        .await?;

    let mut api_postures = Vec::with_capacity(device_postures.len());
    for posture in device_postures {
        let db_rules = DevicePostureOsRule::find_by_posture(&mut *conn, posture.id).await?;
        let locations = DevicePostureLocation::find_by_posture(&mut *conn, posture.id).await?;
        let mut api = ApiDevicePosture::from(posture);
        api.os_rules = db_rules.into_iter().map(ApiOsRule::from).collect();
        api.locations = locations;
        api_postures.push(api);
    }

    Ok(PaginatedApiResponse::new(
        api_postures,
        pagination,
        count as u32,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/device-posture/{id}",
    tag = "DevicePosture",
    params(
        ("id" = Id, Path, description = "Device posture check policy ID")
    ),
    responses(
        (status = 200, description = "Device posture check policy", body = ApiDevicePosture),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn get_device_posture(
    _admin: AdminRole,
    session: SessionInfo,
    Path(id): Path<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!(
        "User {} fetching device posture check {id}",
        session.user.username
    );

    let device_posture = DevicePosture::find_by_id(&appstate.pool, id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("Device posture check {id} not found")))?;

    let db_rules = DevicePostureOsRule::find_by_posture(&appstate.pool, id).await?;
    let locations = DevicePostureLocation::find_by_posture(&appstate.pool, id).await?;
    let mut response = ApiDevicePosture::from(device_posture);
    response.os_rules = db_rules.into_iter().map(ApiOsRule::from).collect();
    response.locations = locations;

    Ok(ApiResponse::json(response, StatusCode::OK))
}

/// Update an existing device posture check policy
#[utoipa::path(
    put,
    path = "/api/v1/device-posture/{id}",
    tag = "DevicePosture",
    params(
        ("id" = Id, Path, description = "Device posture check policy ID")
    ),
    request_body = EditDevicePosture,
    responses(
        (status = 200, description = "Device posture check policy updated successfully", body = ApiDevicePosture),
        (status = 400, description = "Bad request - invalid field value"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - enterprise license required"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn update_device_posture(
    _license: LicenseGated<DevicePostureFeature>,
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(id): Path<Id>,
    State(appstate): State<AppState>,
    Json(data): Json<EditDevicePosture>,
) -> ApiResult {
    debug!(
        "User {} updating device posture check {id}",
        session.user.username
    );

    validate_device_posture_base(&data)?;

    let before_posture = DevicePosture::find_by_id(&appstate.pool, id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("Device posture check {id} not found")))?;
    let before_os_rules = DevicePostureOsRule::find_by_posture(&appstate.pool, id).await?;

    let EditDevicePosture {
        name,
        description,
        min_desktop_client_version,
        min_mobile_client_version,
        allow_prerelease_client,
        os_rules,
    } = data;

    let after = DevicePosture {
        id,
        name,
        description,
        min_desktop_client_version,
        min_mobile_client_version,
        allow_prerelease_client,
    };

    let mut tx = appstate.pool.begin().await?;

    after.save(&mut *tx).await?;
    DevicePostureOsRule::delete_by_posture(&mut *tx, id).await?;
    for rule in &os_rules {
        rule.clone().into_db_rule(id).save(&mut *tx).await?;
    }

    tx.commit().await?;

    let location_ids = DevicePostureLocation::find_by_posture(&appstate.pool, id).await?;

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::DevicePostureUpdated {
            before: DevicePostureSnapshot {
                device_posture: before_posture,
                os_rules: before_os_rules,
                location_ids: location_ids.clone(),
            },
            after: DevicePostureSnapshot {
                device_posture: after.clone(),
                os_rules: DevicePostureOsRule::find_by_posture(&appstate.pool, id).await?,
                location_ids,
            },
        }),
    })?;

    let mut response = ApiDevicePosture::from(after);
    response.os_rules = os_rules;
    Ok(ApiResponse::json(response, StatusCode::OK))
}

/// Delete a device posture check policy
#[utoipa::path(
    delete,
    path = "/api/v1/device-posture/{id}",
    tag = "DevicePosture",
    params(
        ("id" = Id, Path, description = "Device posture check policy ID")
    ),
    responses(
        (status = 200, description = "Device posture check policy deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - enterprise license required"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn delete_device_posture(
    _license: LicenseGated<DevicePostureFeature>,
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(id): Path<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!(
        "User {} deleting device posture check {id}",
        session.user.username
    );

    let device_posture = DevicePosture::find_by_id(&appstate.pool, id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("Device posture check {id} not found")))?;
    let os_rules = DevicePostureOsRule::find_by_posture(&appstate.pool, id).await?;
    let location_ids = DevicePostureLocation::find_by_posture(&appstate.pool, id).await?;

    let mut tx = appstate.pool.begin().await?;
    device_posture.clone().delete(&mut *tx).await?;
    let gateway_commands =
        build_location_peer_refresh_commands(&mut tx, location_ids.clone()).await?;
    tx.commit().await?;

    appstate.send_multiple_gateway_commands(gateway_commands);

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::DevicePostureDeleted {
            snapshot: DevicePostureSnapshot {
                device_posture,
                os_rules,
                location_ids,
            },
        }),
    })?;

    Ok(ApiResponse::default())
}

/// Duplicate a device posture check policy
///
/// Creates a copy of the specified policy with the name `"{original} (copy)"`.
#[utoipa::path(
    post,
    path = "/api/v1/device-posture/{id}/duplicate",
    tag = "DevicePosture",
    params(
        ("id" = Id, Path, description = "Device posture check policy ID to duplicate")
    ),
    responses(
        (status = 201, description = "Duplicate created successfully", body = ApiDevicePosture),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - enterprise license required"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn duplicate_device_posture(
    _license: LicenseGated<DevicePostureFeature>,
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(id): Path<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!(
        "User {} duplicating device posture check {id}",
        session.user.username
    );

    let original = DevicePosture::find_by_id(&appstate.pool, id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("Device posture check {id} not found")))?;

    let original_rules = DevicePostureOsRule::find_by_posture(&appstate.pool, id).await?;

    let mut tx = appstate.pool.begin().await?;

    let duplicate = DevicePosture {
        id: NoId,
        name: format!("Copy of {}", original.name),
        description: original.description.clone(),
        min_desktop_client_version: original.min_desktop_client_version.clone(),
        min_mobile_client_version: original.min_mobile_client_version.clone(),
        allow_prerelease_client: original.allow_prerelease_client,
    }
    .save(&mut *tx)
    .await?;

    for rule in &original_rules {
        ApiOsRule::from(rule.clone())
            .into_db_rule(duplicate.id)
            .save(&mut *tx)
            .await?;
    }

    let original_location_ids = DevicePostureLocation::find_by_posture(&appstate.pool, id).await?;

    tx.commit().await?;

    let duplicate_rules =
        DevicePostureOsRule::find_by_posture(&appstate.pool, duplicate.id).await?;

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::DevicePostureDuplicated {
            original: DevicePostureSnapshot {
                device_posture: original,
                os_rules: original_rules.clone(),
                location_ids: original_location_ids,
            },
            duplicate: DevicePostureSnapshot {
                device_posture: duplicate.clone(),
                os_rules: duplicate_rules,
                location_ids: Vec::new(),
            },
        }),
    })?;

    let mut response = ApiDevicePosture::from(duplicate);
    response.os_rules = original_rules.into_iter().map(ApiOsRule::from).collect();
    Ok(ApiResponse::json(response, StatusCode::CREATED))
}

/// Request body for assigning posture checks to a VPN location.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AssignPosturesData {
    pub postures: Vec<Id>,
}

/// Request body for assigning VPN locations to a posture check.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AssignLocationsData {
    pub locations: Vec<Id>,
}

/// Assign posture checks to a VPN location (replaces existing assignment)
#[utoipa::path(
    put,
    path = "/api/v1/network/{id}/postures",
    tag = "DevicePosture",
    params(
        ("id" = Id, Path, description = "VPN location ID")
    ),
    request_body = AssignPosturesData,
    responses(
        (status = 200, description = "Postures assigned successfully", body = [Id]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - enterprise license required"),
        (status = 404, description = "Location not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn set_postures_for_location(
    _license: LicenseGated<DevicePostureFeature>,
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(location_id): Path<Id>,
    State(appstate): State<AppState>,
    Json(data): Json<AssignPosturesData>,
) -> ApiResult {
    debug!(
        "User {} assigning device posture checks {:?} to location {location_id}",
        session.user.username, data.postures
    );

    let location = WireguardNetwork::find_by_id(&appstate.pool, location_id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("Location {location_id} not found")))?;

    if location.is_service_location() && !data.postures.is_empty() {
        return Err(WebError::BadRequest(
            "Posture checks cannot be assigned to service locations".to_owned(),
        ));
    }

    let mut tx = appstate.pool.begin().await?;
    let old_postures = DevicePostureLocation::find_by_location(&mut *tx, location_id).await?;
    let result =
        DevicePostureLocation::set_for_location(&mut tx, location_id, &data.postures).await?;
    let gateway_commands = if same_id_set(&old_postures, &result) {
        Vec::new()
    } else {
        build_location_peer_refresh_commands(&mut tx, [location_id]).await?
    };
    tx.commit().await?;

    appstate.send_multiple_gateway_commands(gateway_commands);

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::LocationPosturesAssigned {
            location,
            posture_ids: result.clone(),
        }),
    })?;

    Ok(ApiResponse::json(result, StatusCode::OK))
}

/// Assign VPN locations to a posture check (replaces existing assignment)
#[utoipa::path(
    put,
    path = "/api/v1/device-posture/{id}/locations",
    tag = "DevicePosture",
    params(
        ("id" = Id, Path, description = "Device posture check policy ID")
    ),
    request_body = AssignLocationsData,
    responses(
        (status = 200, description = "Locations assigned successfully", body = [Id]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - enterprise license required"),
        (status = 404, description = "Posture check not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn set_locations_for_posture(
    _license: LicenseGated<DevicePostureFeature>,
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(posture_id): Path<Id>,
    State(appstate): State<AppState>,
    Json(data): Json<AssignLocationsData>,
) -> ApiResult {
    debug!(
        "User {} assigning locations {:?} to device posture check {posture_id}",
        session.user.username, data.locations
    );

    let posture = DevicePosture::find_by_id(&appstate.pool, posture_id)
        .await?
        .ok_or_else(|| {
            WebError::ObjectNotFound(format!("Device posture check {posture_id} not found"))
        })?;

    for location_id in &data.locations {
        if let Some(location) = WireguardNetwork::find_by_id(&appstate.pool, *location_id).await?
            && location.is_service_location()
        {
            return Err(WebError::BadRequest(
                "Posture checks cannot be assigned to service locations".to_owned(),
            ));
        }
    }

    let mut tx = appstate.pool.begin().await?;
    let old_locations = DevicePostureLocation::find_by_posture(&mut *tx, posture_id).await?;
    let result =
        DevicePostureLocation::set_for_posture(&mut tx, posture_id, &data.locations).await?;
    let gateway_commands = if same_id_set(&old_locations, &result) {
        Vec::new()
    } else {
        build_location_peer_refresh_commands(
            &mut tx,
            old_locations.iter().copied().chain(result.iter().copied()),
        )
        .await?
    };
    tx.commit().await?;

    appstate.send_multiple_gateway_commands(gateway_commands);

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::DevicePostureLocationsAssigned {
            device_posture: posture,
            location_ids: result.clone(),
        }),
    })?;

    Ok(ApiResponse::json(result, StatusCode::OK))
}
