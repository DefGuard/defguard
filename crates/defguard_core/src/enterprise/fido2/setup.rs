use defguard_common::db::{
    Id,
    models::{Settings, WebAuthn, biometric_auth::BiometricChallenge, user::User},
};
use sqlx::{PgExecutor, PgPool, query, types::Uuid};
use webauthn_rs::prelude::{PasskeyRegistration, RegisterPublicKeyCredential};

use crate::db::models::enrollment::{MfaConfigAuthState, Token, TokenError};

impl Token {
    /// Token-based counterpart of `Session::set_passkey_registration` on the REST path.
    pub async fn set_passkey_registration<'e, E>(
        &mut self,
        executor: E,
        passkey_reg: &PasskeyRegistration,
    ) -> Result<(), TokenError>
    where
        E: PgExecutor<'e>,
    {
        let mfa_setup_state = serde_cbor::to_vec(passkey_reg)
            .map_err(|err| TokenError::MfaSetupStateSerialization(err.to_string()))?;
        query!(
            "UPDATE token SET mfa_setup_state = $1 WHERE id = $2",
            mfa_setup_state,
            self.id
        )
        .execute(executor)
        .await?;
        self.mfa_setup_state = Some(mfa_setup_state);
        Ok(())
    }

    #[must_use]
    pub fn get_passkey_registration(&self) -> Option<PasskeyRegistration> {
        self.mfa_setup_state
            .as_ref()
            .and_then(|state| serde_cbor::from_slice(state).ok())
    }

    /// Stores a new single-use challenge for authorizing an MFA configuration session with FIDO2.
    /// Returns `None`, storing nothing, when a completed OIDC attempt is waiting for the client to
    /// authorize the session.
    pub async fn issue_fido2_challenge(
        &mut self,
        pool: &PgPool,
    ) -> Result<Option<String>, TokenError> {
        let challenge = BiometricChallenge::new().challenge;
        if !self
            .replace_mfa_config_auth_state(
                pool,
                &MfaConfigAuthState::Fido2 {
                    challenge: challenge.clone(),
                },
            )
            .await?
        {
            return Ok(None);
        }
        Ok(Some(challenge))
    }

    /// Removes and returns the stored FIDO2 challenge, so each challenge verifies at most once.
    pub async fn take_fido2_challenge(
        &mut self,
        pool: &PgPool,
    ) -> Result<Option<String>, TokenError> {
        let mut transaction = pool.begin().await?;
        let Some(MfaConfigAuthState::Fido2 { challenge }) =
            self.lock_mfa_config_auth_state(&mut transaction).await?
        else {
            return Ok(None);
        };
        query!(
            "UPDATE token SET mfa_setup_state = NULL WHERE id = $1",
            self.id
        )
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.mfa_setup_state = None;
        Ok(Some(challenge))
    }

    /// Like the REST `webauthn_init`, but stores the ceremony state on the token.
    /// Returns the `CreationChallengeResponse` as JSON for the client's authenticator.
    pub async fn start_fido2_setup(
        &mut self,
        pool: &PgPool,
        user: &User<Id>,
    ) -> Result<String, TokenError> {
        let passkeys = WebAuthn::passkeys_for_user(pool, user.id).await?;
        let webauthn = Settings::get_current_settings()
            .build_webauthn()
            .map_err(|err| TokenError::WebauthnConfig(err.to_string()))?;
        let (ccr, passkey_reg) = webauthn
            .start_passkey_registration(
                Uuid::new_v4(),
                &user.username,
                &user.username,
                Some(passkeys.iter().map(|key| key.cred_id().clone()).collect()),
            )
            .map_err(|err| TokenError::WebauthnRegistration(err.to_string()))?;
        self.set_passkey_registration(pool, &passkey_reg).await?;
        serde_json::to_string(&ccr).map_err(|err| TokenError::WebauthnRegistration(err.to_string()))
    }

    /// Like the REST `webauthn_finish`: verifies the attestation against the stored
    /// challenge, saves the security key and clears the ceremony state.
    pub async fn finish_fido2_setup(
        &mut self,
        pool: &PgPool,
        user_id: Id,
        name: String,
        attestation_json: &str,
    ) -> Result<WebAuthn<Id>, TokenError> {
        let webauthn = Settings::get_current_settings()
            .build_webauthn()
            .map_err(|err| TokenError::WebauthnConfig(err.to_string()))?;
        let passkey_reg = self.get_passkey_registration().ok_or_else(|| {
            TokenError::WebauthnRegistration("Passkey registration session not found".into())
        })?;
        let rpkc: RegisterPublicKeyCredential =
            serde_json::from_str(attestation_json).map_err(|_| {
                TokenError::WebauthnRegistration("Failed to parse registration attestation".into())
            })?;
        let passkey = webauthn
            .finish_passkey_registration(&rpkc, &passkey_reg)
            .map_err(|err| TokenError::WebauthnRegistration(err.to_string()))?;
        let webauthn_key = WebAuthn::new(user_id, name, &passkey)
            .map_err(|err| TokenError::WebauthnRegistration(err.to_string()))?
            .save(pool)
            .await
            .map_err(|err| TokenError::WebauthnRegistration(err.to_string()))?;
        self.clear_mfa_setup_state(pool).await?;
        Ok(webauthn_key)
    }
}
