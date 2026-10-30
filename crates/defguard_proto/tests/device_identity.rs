use defguard_proto::{
    client_types::{
        DeviceConfigResponse, EnrollmentStartRequest, InstanceInfoRequest, NewDevice,
        enrollment_start_request,
    },
    device_identity::{
        AttestationIdentity, AttestationResponse, IdentityState, PossessionChallenge,
        PossessionResponse, Signature, SignatureAlgorithm, TpmAttestationIdentity,
        TpmAttestationResponse, attestation_identity, attestation_response, possession_response,
    },
    proxy::{CoreRequest, CoreResponse, core_request, core_response},
};
use prost::Message;

#[test]
fn test_device_identity_requests_roundtrip_in_core_envelopes() {
    let requests = [
        core_request::Payload::EnrollmentStart(EnrollmentStartRequest {
            token: "enrollment-token".into(),
            proposed_wireguard_pubkey: Some("wireguard-key".into()),
            device_identity: Some(
                enrollment_start_request::DeviceIdentity::AttestationIdentity(
                    AttestationIdentity {
                        backend: Some(attestation_identity::Backend::Tpm(TpmAttestationIdentity {
                            ek_public: vec![0, 255],
                            ak_public: vec![128, 1],
                            ..Default::default()
                        })),
                    },
                ),
            ),
        }),
        core_request::Payload::NewDevice(NewDevice {
            attestation_response: Some(AttestationResponse {
                session_id: "session".into(),
                result: Some(attestation_response::Result::Tpm(TpmAttestationResponse {
                    activation_secret: vec![0, 255, 128],
                    evidence: None,
                })),
            }),
            ..Default::default()
        }),
        core_request::Payload::InstanceInfo(InstanceInfoRequest {
            token: "polling-token".into(),
            possession_response: Some(PossessionResponse {
                identity_id: "identity".into(),
                challenge_id: "challenge".into(),
                result: Some(possession_response::Result::Signature(Signature {
                    algorithm: SignatureAlgorithm::EcdsaSha256 as i32,
                    signature: vec![0, 255, 128],
                })),
            }),
        }),
    ];
    for payload in requests {
        let request = CoreRequest {
            id: 1,
            payload: Some(payload),
            ..Default::default()
        };
        let encoded = request.encode_to_vec();
        let decoded = CoreRequest::decode(encoded.as_slice()).unwrap();
        assert!(decoded == request);
    }
}

#[test]
fn test_device_identity_state_and_challenge_roundtrip_in_core_response() {
    let response = CoreResponse {
        id: 1,
        payload: Some(core_response::Payload::DeviceConfig(DeviceConfigResponse {
            identity_state: Some(IdentityState {
                identity_id: "identity".into(),
                key_fingerprint: vec![0, 255, 128],
                ..Default::default()
            }),
            possession_challenge: Some(PossessionChallenge {
                challenge_id: "challenge".into(),
                data: vec![0, 255, 128],
                expires_at: 1_800_000_000,
            }),
            ..Default::default()
        })),
    };
    let encoded = response.encode_to_vec();
    let decoded = CoreResponse::decode(encoded.as_slice()).unwrap();
    assert!(decoded == response);
}
