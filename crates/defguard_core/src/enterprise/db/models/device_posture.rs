use defguard_common::db::{Id, NoId};
use model_derive::Model;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Device posture check policy. Defines the security requirements a client
/// device must satisfy before being allowed to connect to an assigned VPN location.
#[derive(Clone, Debug, Deserialize, Model, Serialize, ToSchema, PartialEq)]
#[table(device_posture)]
pub struct DevicePosture<I = NoId> {
    pub id: I,
    pub name: String,
    pub description: Option<String>,
    pub min_client_version: Option<String>,
    pub allow_prerelease_client: bool,
}
