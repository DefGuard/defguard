mod challenge_data;
mod session;
pub mod tpm;

pub use challenge_data::{AttestationData, qualifying_data};

use chrono::Utc;
use defguard_common::db::{
    Id,
    models::{Device, Settings, device_identity::DeviceAttestationStatus},
};
use defguard_proto::device_identity::{
    AttestationChallenge, AttestationChallengeSolution, AttestationIdentity, AttestationStatus,
    IdentityState, attestation_challenge, attestation_challenge_solution, attestation_identity,
};
use sqlx::PgPool;
use tonic::Status;

use self::{session::lock_attestation_session, tpm::TpmError};
use crate::{
    db::models::enrollment::Token,
    enterprise::db::models::device_identity::{
        DeviceAttestationChallenge, DeviceAttestationPayload, DeviceAttestationState,
        DeviceIdentity, DeviceIdentityBackend,
    },
};

fn database_error(error: sqlx::Error) -> Status {
    error!("Device attestation database operation failed: {error}");
    Status::internal("device attestation database operation failed")
}

async fn record_attestation_failure(
    pool: &PgPool,
    enrollment: &Token,
    device: &Device<Id>,
    state: &DeviceAttestationState,
) -> Result<(), Status> {
    let mut transaction = pool.begin().await.map_err(database_error)?;
    lock_attestation_session(&mut transaction, enrollment, device).await?;
    if !state
        .consume(&mut *transaction)
        .await
        .map_err(database_error)?
    {
        return Err(Status::failed_precondition(
            "attestation challenge expired, changed or was already consumed",
        ));
    }
    sqlx::query!(
        "UPDATE device SET attestation_status = $1, attested_at = NULL \
         WHERE id = $2 AND attestation_status = $3",
        DeviceAttestationStatus::AttestationFailed as DeviceAttestationStatus,
        device.id,
        DeviceAttestationStatus::NotAttested as DeviceAttestationStatus,
    )
    .execute(&mut *transaction)
    .await
    .map_err(database_error)?;
    transaction.commit().await.map_err(database_error)
}

fn crypto_error(error: TpmError) -> Status {
    match error {
        TpmError::Crypto(error) => {
            error!("Device attestation cryptography failed: {error}");
            Status::internal("device attestation cryptography failed")
        }
        TpmError::UnsupportedProfile(message) => Status::invalid_argument(message),
        TpmError::InvalidPublic(message) | TpmError::InvalidEvidence(message) => {
            Status::invalid_argument(message)
        }
    }
}

/// Generates and persists a fresh TPM challenge for an existing device and enrollment session.
#[instrument(skip_all)]
pub async fn generate_attestation_challenge(
    pool: &PgPool,
    enrollment: &Token,
    device: &Device<Id>,
    identity: &AttestationIdentity,
) -> Result<AttestationChallenge, Status> {
    let identity = match identity.backend.as_ref() {
        Some(attestation_identity::Backend::Tpm(identity)) => identity.clone(),
        Some(attestation_identity::Backend::AppleAppAttest(_)) => {
            return Err(Status::unimplemented("Apple App Attest is not supported"));
        }
        None => return Err(Status::invalid_argument("missing attestation backend")),
    };
    let now = Utc::now().naive_utc();
    let data = AttestationData::new(
        enrollment,
        device,
        Settings::get_current_settings().enrollment_session_timeout(),
        now,
    )?;
    let prepared = tokio::task::spawn_blocking(move || tpm::create_challenge(&identity))
        .await
        .map_err(|_| Status::internal("attestation challenge generation task failed"))?
        .map_err(crypto_error)?;
    let challenge = AttestationChallenge {
        session_id: data.challenge_id().into(),
        data: data.as_bytes().to_vec(),
        challenge: Some(attestation_challenge::Challenge::Tpm(prepared.challenge)),
        expires_at: data.expires_at().and_utc().timestamp(),
    };
    let state = DeviceAttestationState {
        challenge_id: data.challenge_id().into(),
        token_id: enrollment.id.clone(),
        device_id: device.id,
        wireguard_key: device.wireguard_pubkey.clone(),
        created_at: now,
        expires_at: data.expires_at(),
        payload: DeviceAttestationPayload {
            data: data.into_bytes(),
            backend: prepared.verification,
        },
    };
    let mut transaction = pool.begin().await.map_err(database_error)?;
    let now = lock_attestation_session(&mut transaction, enrollment, device).await?;
    if now >= state.expires_at {
        return Err(Status::deadline_exceeded(
            "attestation challenge has expired",
        ));
    }
    state
        .save(&mut *transaction)
        .await
        .map_err(database_error)?;
    transaction.commit().await.map_err(database_error)?;
    Ok(challenge)
}

/// Verifies a pending TPM challenge and atomically registers the device's first identity.
#[instrument(skip_all)]
pub async fn verify_attestation(
    pool: &PgPool,
    enrollment: &Token,
    device: &Device<Id>,
    solution: &AttestationChallengeSolution,
) -> Result<IdentityState, Status> {
    let response = match solution.result.as_ref() {
        Some(attestation_challenge_solution::Result::Tpm(response)) => response.clone(),
        Some(attestation_challenge_solution::Result::AppleAppAttest(_)) => {
            return Err(Status::unimplemented("Apple App Attest is not supported"));
        }
        Some(attestation_challenge_solution::Result::Error(_)) => {
            return Err(Status::failed_precondition(
                "client could not complete device attestation",
            ));
        }
        None => return Err(Status::invalid_argument("missing attestation result")),
    };
    let state = DeviceAttestationState::find_for_device(pool, &enrollment.id, device.id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| Status::failed_precondition("no pending attestation challenge"))?;
    if state.challenge_id != solution.session_id || state.wireguard_key != device.wireguard_pubkey {
        return Err(Status::invalid_argument(
            "attestation challenge does not match device or session",
        ));
    }
    if Utc::now().naive_utc() >= state.expires_at {
        return Err(Status::deadline_exceeded(
            "attestation challenge has expired",
        ));
    }
    if !matches!(
        state.payload.backend,
        DeviceAttestationChallenge::Tpm { .. }
    ) {
        return Err(Status::invalid_argument(
            "attestation backend does not match challenge",
        ));
    }
    let payload = state.payload.clone();
    let verified = tokio::task::spawn_blocking(move || {
        tpm::verify_attestation(&payload.backend, &payload.data, &response)
    })
    .await
    .map_err(|_| Status::internal("attestation verification task failed"))?;
    let verified = match verified {
        Ok(verified) => verified,
        Err(error @ (TpmError::InvalidPublic(_) | TpmError::InvalidEvidence(_))) => {
            record_attestation_failure(pool, enrollment, device, &state).await?;
            return Err(crypto_error(error));
        }
        Err(error) => return Err(crypto_error(error)),
    };
    let fingerprint = verified.public_key_fingerprint().to_vec();
    let identity = DeviceIdentity {
        device_id: device.id,
        backend: DeviceIdentityBackend::Tpm,
        public_key_der: verified.public_key_der,
        apple_sign_count: None,
        tpm_ek_public: Some(verified.ek_public),
        tpm_ek_certificate_chain_der: Some(verified.ek_certificate_chain_der),
        apple_key_id: None,
    };
    let mut transaction = pool.begin().await.map_err(database_error)?;
    let now = lock_attestation_session(&mut transaction, enrollment, device).await?;
    if !state
        .consume(&mut *transaction)
        .await
        .map_err(database_error)?
    {
        return Err(Status::failed_precondition(
            "attestation challenge expired, changed or was already consumed",
        ));
    }
    let device = identity
        .save_verified(&mut transaction, DeviceAttestationStatus::Attested, now)
        .await
        .map_err(|error| {
            error!("Failed to register attested device identity: {error}");
            Status::internal("failed to register device identity")
        })?;
    transaction.commit().await.map_err(database_error)?;
    Ok(IdentityState {
        identity_id: device.id.to_string(),
        attestation_status: AttestationStatus::Attested.into(),
        error: None,
        key_fingerprint: fingerprint,
        attested_at: device.attested_at.map(|time| time.and_utc().timestamp()),
        last_possession_verified_at: None,
    })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod integration_tests;
