use std::{collections::BTreeMap, time::Duration};

use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::{DateTime, NaiveDateTime, TimeDelta};
use defguard_common::db::models::{Device, DeviceType};
use serde_cbor::Value;
use tonic::Code;

use super::{AttestationData, qualifying_data};
use crate::db::models::enrollment::{ENROLLMENT_TOKEN_TYPE, Token};

const SESSION_TIMEOUT: Duration = Duration::from_secs(1800);
const WIREGUARD_KEY: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";

fn now() -> NaiveDateTime {
    DateTime::from_timestamp(1_800_000_000, 750_000_000)
        .unwrap()
        .naive_utc()
}

fn enrollment() -> Token {
    let mut token = Token::new(7, None, None, 3600, Some(ENROLLMENT_TOKEN_TYPE.into()));
    token.id = "enrollment-token-for-payload".into();
    token.used_at = Some(now() - TimeDelta::minutes(5));
    token.expires_at = now() + TimeDelta::hours(1);
    token
}

fn device() -> Device<i64> {
    Device::new(
        "laptop".into(),
        WIREGUARD_KEY.into(),
        7,
        DeviceType::User,
        None,
        true,
    )
    .with_id(42)
}

fn fields(data: &AttestationData) -> BTreeMap<Value, Value> {
    let Value::Map(fields) = serde_cbor::from_slice(data.as_bytes()).unwrap() else {
        panic!("attestation payload must be a CBOR map");
    };
    fields
}

fn bytes_field(data: &AttestationData, name: &str) -> Vec<u8> {
    let Value::Bytes(bytes) = fields(data).remove(&Value::Text(name.into())).unwrap() else {
        panic!("{name} must be a CBOR byte string");
    };
    bytes
}

#[test]
fn test_attestation_data_binds_the_operation_session_and_device() {
    let token = enrollment();
    let data = AttestationData::new(&token, &device(), SESSION_TIMEOUT, now()).unwrap();
    let fields = fields(&data);
    assert_eq!(fields.len(), 8);
    for (name, expected) in [
        ("version", Value::Integer(1)),
        (
            "purpose",
            Value::Text("defguard.device-identity.attestation".into()),
        ),
        ("challenge_id", Value::Text(data.challenge_id().into())),
        ("device_id", Value::Integer(42)),
        ("wireguard_public_key", Value::Bytes((0..32).collect())),
        ("expires_at", Value::Integer(1_800_001_500)),
    ] {
        assert_eq!(
            fields.get(&Value::Text(name.into())),
            Some(&expected),
            "{name}"
        );
    }
    assert_eq!(
        bytes_field(&data, "session_binding"),
        vec![
            84, 160, 242, 10, 190, 45, 246, 184, 120, 115, 233, 119, 231, 134, 164, 128, 140, 179,
            5, 207, 109, 117, 70, 35, 192, 160, 146, 181, 16, 32, 36, 101,
        ]
    );
    assert!(
        !data
            .as_bytes()
            .windows(token.id.len())
            .any(|bytes| bytes == token.id.as_bytes())
    );
    assert_eq!(bytes_field(&data, "nonce").len(), 32);
    assert_eq!(data.expires_at().and_utc().timestamp(), 1_800_001_500);
    let issued_bytes = data.as_bytes().to_vec();
    assert_eq!(data.into_bytes(), issued_bytes);
}

#[test]
fn test_attestation_data_has_a_fresh_challenge_and_nonce_for_each_attempt() {
    let token = enrollment();
    let device = device();
    let first = AttestationData::new(&token, &device, SESSION_TIMEOUT, now()).unwrap();
    let second = AttestationData::new(&token, &device, SESSION_TIMEOUT, now()).unwrap();
    assert_ne!(first.challenge_id(), second.challenge_id());
    assert_ne!(bytes_field(&first, "nonce"), bytes_field(&second, "nonce"));
    assert_ne!(first.as_bytes(), second.as_bytes());
}

#[test]
fn test_attestation_data_uses_the_session_deadline_not_invitation_expiry() {
    let mut token = enrollment();
    token.expires_at = now() - TimeDelta::seconds(1);
    let data = AttestationData::new(&token, &device(), SESSION_TIMEOUT, now()).unwrap();
    assert_eq!(data.expires_at().and_utc().timestamp(), 1_800_001_500);
    assert!(data.expires_at() <= token.used_at.unwrap() + TimeDelta::minutes(30));
    let last_valid_instant = data.expires_at() - TimeDelta::nanoseconds(1);
    assert!(AttestationData::new(&token, &device(), SESSION_TIMEOUT, last_valid_instant).is_ok());
    assert_eq!(
        AttestationData::new(&token, &device(), SESSION_TIMEOUT, data.expires_at())
            .err()
            .unwrap()
            .code(),
        Code::Unauthenticated
    );
}

#[test]
fn test_attestation_data_requires_a_started_unexpired_session() {
    let mut token = enrollment();
    token.used_at = None;
    assert_eq!(
        AttestationData::new(&token, &device(), SESSION_TIMEOUT, now())
            .err()
            .unwrap()
            .code(),
        Code::Unauthenticated
    );
    token.used_at = Some(now() - TimeDelta::hours(1));
    assert_eq!(
        AttestationData::new(&token, &device(), SESSION_TIMEOUT, now())
            .err()
            .unwrap()
            .code(),
        Code::Unauthenticated
    );
    token.used_at = Some(now());
    assert_eq!(
        AttestationData::new(&token, &device(), Duration::MAX, now())
            .err()
            .unwrap()
            .code(),
        Code::Internal
    );
}

#[test]
fn test_attestation_data_rejects_invalid_wireguard_public_keys() {
    for key in [
        "not-base64".into(),
        BASE64_STANDARD.encode([1; 31]),
        BASE64_STANDARD.encode([1; 33]),
    ] {
        let mut device = device();
        device.wireguard_pubkey = key;
        assert_eq!(
            AttestationData::new(&enrollment(), &device, SESSION_TIMEOUT, now())
                .err()
                .unwrap()
                .code(),
            Code::InvalidArgument
        );
    }
}

#[test]
fn test_qualifying_data_hashes_the_exact_issued_bytes() {
    assert_eq!(
        qualifying_data(&[0, 255, 128, 1]),
        [
            101, 9, 66, 63, 217, 218, 92, 34, 93, 47, 134, 25, 255, 174, 57, 75, 64, 249, 247, 104,
            111, 238, 85, 163, 140, 84, 177, 66, 74, 198, 95, 70,
        ]
    );
    assert_ne!(
        qualifying_data(&[0, 255, 128, 1]),
        qualifying_data(&[0, 255, 128, 2])
    );
    let data = AttestationData::new(&enrollment(), &device(), SESSION_TIMEOUT, now()).unwrap();
    let fields = fields(&data);
    let reordered_bytes = serde_cbor::to_vec(&fields).unwrap();
    assert_ne!(data.as_bytes(), reordered_bytes);
    assert_ne!(
        qualifying_data(data.as_bytes()),
        qualifying_data(&reordered_bytes)
    );
}
