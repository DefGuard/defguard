use serde_cbor::Value;
use webauthn_rs::prelude::RegisterPublicKeyCredential;

const RP_ID_HASH_LEN: usize = 32;
const SIGN_COUNT_LEN: usize = 4;
const MIN_AUTH_DATA_LEN: usize = RP_ID_HASH_LEN + 1 + SIGN_COUNT_LEN;

const USER_PRESENT: u8 = 0x01;
const USER_VERIFIED: u8 = 0x04;
const BACKUP_ELIGIBLE: u8 = 0x08;
const BACKUP_STATE: u8 = 0x10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthFlags {
    pub user_present: bool,
    pub user_verified: bool,
    pub backup_eligible: bool,
    pub backup_state: bool,
}

impl AuthFlags {
    #[must_use]
    pub fn parse(auth_data: &[u8]) -> Option<Self> {
        if auth_data.len() < MIN_AUTH_DATA_LEN {
            return None;
        }
        let flags = auth_data[RP_ID_HASH_LEN];
        Some(Self {
            user_present: flags & USER_PRESENT != 0,
            user_verified: flags & USER_VERIFIED != 0,
            backup_eligible: flags & BACKUP_ELIGIBLE != 0,
            backup_state: flags & BACKUP_STATE != 0,
        })
    }

    #[must_use]
    pub fn from_registration(credential: &RegisterPublicKeyCredential) -> Option<Self> {
        let Value::Map(object) =
            serde_cbor::from_slice(&credential.response.attestation_object).ok()?
        else {
            return None;
        };
        match object.get(&Value::Text("authData".into()))? {
            Value::Bytes(auth_data) => Self::parse(auth_data),
            _ => None,
        }
    }

    #[must_use]
    pub fn present_and_verified(self) -> bool {
        self.user_present && self.user_verified
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth_data(flags: u8) -> Vec<u8> {
        let mut data = vec![0; MIN_AUTH_DATA_LEN];
        data[RP_ID_HASH_LEN] = flags;
        data
    }

    #[test]
    fn parses_flags() {
        let flags = AuthFlags::parse(&auth_data(0x1d)).unwrap();
        assert_eq!(
            flags,
            AuthFlags {
                user_present: true,
                user_verified: true,
                backup_eligible: true,
                backup_state: true,
            }
        );
        assert!(flags.present_and_verified());
    }

    #[test]
    fn requires_both_presence_and_verification() {
        assert!(
            !AuthFlags::parse(&auth_data(0x01))
                .unwrap()
                .present_and_verified()
        );
        assert!(
            !AuthFlags::parse(&auth_data(0x04))
                .unwrap()
                .present_and_verified()
        );
        assert!(
            AuthFlags::parse(&auth_data(0x05))
                .unwrap()
                .present_and_verified()
        );
    }

    #[test]
    fn rejects_truncated_auth_data() {
        assert_eq!(AuthFlags::parse(&[0; MIN_AUTH_DATA_LEN - 1]), None);
    }
}
