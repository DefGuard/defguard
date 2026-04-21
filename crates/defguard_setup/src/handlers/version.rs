use axum::{Extension, Json};
use semver::Version;
use serde::Serialize;

#[derive(Serialize)]
pub struct VersionResponse {
    version: String,
}

pub async fn get_version(Extension(version): Extension<Version>) -> Json<VersionResponse> {
    Json(VersionResponse {
        version: version.to_string(),
    })
}
