use crate::db::{Device, Id, MFAMethod, WireguardNetwork};

#[derive(Serialize)]
pub struct MfaLoginMetadata {
    pub mfa_method: MFAMethod,
}

#[derive(Serialize)]
pub struct DeviceAddedMetadata {
    pub device_names: Vec<String>,
}

#[derive(Serialize)]
pub struct DeviceRemovedMetadata {
    pub device_names: Vec<String>,
}

#[derive(Serialize)]
pub struct DeviceModifiedMetadata {
    pub device_names: Vec<String>,
}

#[derive(Serialize)]
pub struct NetworkDeviceAddedMetadata {
    pub device_id: Id,
    pub device_name: String,
    pub location_id: Id,
    pub location: String,
}

#[derive(Serialize)]
pub struct NetworkDeviceRemovedMetadata {
    pub device_id: Id,
    pub device_name: String,
    pub location_id: Id,
    pub location: String,
}

#[derive(Serialize)]
pub struct NetworkDeviceModifiedMetadata {
    pub device_id: Id,
    pub device_name: String,
    pub location_id: Id,
    pub location: String,
}

#[derive(Serialize)]
pub struct UserAddedMetadata {
    pub username: String,
}

#[derive(Serialize)]
pub struct UserModifiedMetadata {
    pub username: String,
}

#[derive(Serialize)]
pub struct UserRemovedMetadata {
    pub username: String,
}

#[derive(Serialize)]
pub struct MfaSecurityKeyRemovedMetadata {
    pub key_id: Id,
    pub key_name: String,
}

#[derive(Serialize)]
pub struct MfaSecurityKeyAddedMetadata {
    pub key_id: Id,
    pub key_name: String,
}

#[derive(Serialize)]
pub struct ActivityLogStreamMetadata {
    pub id: Id,
    pub name: String,
}

#[derive(Serialize)]
pub struct VpnClientMetadata {
    pub location: WireguardNetwork<Id>,
    pub device: Device<Id>,
}
