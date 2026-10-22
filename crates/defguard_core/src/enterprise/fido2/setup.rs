use defguard_common::db::{
    Id,
    models::{
        Settings, WebAuthn,
        user::User,
        webauthn::{AuthenticatorInfo, AuthenticatorKind, RegistrationSource},
    },
};
use sqlx::{PgExecutor, PgPool, query, types::Uuid};
use webauthn_rs::prelude::{
    AttestedPasskeyRegistration, AuthenticatorAttachment, Passkey, RegisterPublicKeyCredential,
    WebauthnError,
};

use crate::{
    db::models::enrollment::{MfaConfigAuthState, Token, TokenError},
    fido2::attestation::{trusted_attestation_cas, verify_hardware_key},
};

impl Token {
    /// Token-based counterpart of `Session::set_passkey_registration` on the REST path.
    pub async fn set_passkey_registration<'e, E>(
        &mut self,
        executor: E,
        passkey_reg: &AttestedPasskeyRegistration,
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
    pub fn get_passkey_registration(&self) -> Option<AttestedPasskeyRegistration> {
        self.mfa_setup_state
            .as_ref()
            .and_then(|state| serde_cbor::from_slice(state).ok())
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

    /// Like the REST WebAuthn init, but stores the ceremony state on the token and requires
    /// attestation by a trusted hardware authenticator.
    pub async fn start_fido2_setup(
        &mut self,
        pool: &PgPool,
        user: &User<Id>,
    ) -> Result<String, TokenError> {
        let passkeys = WebAuthn::passkeys_for_user(pool, user.id, None).await?;
        let webauthn = Settings::get_current_settings()
            .build_webauthn()
            .map_err(|err| TokenError::WebauthnConfig(err.to_string()))?;
        let (ccr, passkey_reg) = webauthn
            .start_attested_passkey_registration(
                Uuid::new_v4(),
                &user.username,
                &user.username,
                Some(passkeys.iter().map(|key| key.cred_id().clone()).collect()),
                trusted_attestation_cas().into_owned(),
                Some(AuthenticatorAttachment::CrossPlatform),
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
        source: RegistrationSource,
    ) -> Result<WebAuthn<Id>, TokenError> {
        let webauthn = Settings::get_current_settings()
            .build_webauthn()
            .map_err(|err| TokenError::WebauthnConfig(err.to_string()))?;
        let passkey_reg = self.get_passkey_registration().ok_or_else(|| {
            TokenError::WebauthnRegistration(
                "FIDO2 setup session not found or expired, restart the setup".into(),
            )
        })?;
        // The challenge is single use, a failed attempt must restart the ceremony.
        self.clear_mfa_setup_state(pool).await?;
        let rpkc: RegisterPublicKeyCredential =
            serde_json::from_str(attestation_json).map_err(|_| {
                TokenError::WebauthnRegistration("Failed to parse registration attestation".into())
            })?;
        let attested = webauthn
            .finish_attested_passkey_registration(&rpkc, &passkey_reg)
            .map_err(|err| match err {
                WebauthnError::AttestationNotVerifiable
                | WebauthnError::AttestationChainNotTrusted(_)
                | WebauthnError::AttestationLeafCertMissing
                | WebauthnError::AttestationUntrustedAaguid
                | WebauthnError::AttestationNotSupported
                | WebauthnError::AttestationFormatMissingAaguid
                | WebauthnError::AttestationCertificateAAGUIDMismatch
                | WebauthnError::CredentialMayNotBeHardwareBound => {
                    TokenError::UntrustedAuthenticator(err.to_string())
                }
                _ => TokenError::WebauthnRegistration(err.to_string()),
            })?;
        let hardware_key = verify_hardware_key(&attested)
            .map_err(|err| TokenError::UntrustedAuthenticator(err.to_string()))?;
        info!(
            "Security key attested as {} ({})",
            hardware_key
                .description
                .as_deref()
                .unwrap_or("unnamed model"),
            hardware_key.aaguid
        );
        let authenticator = AuthenticatorInfo {
            name: hardware_key.description,
            kind: AuthenticatorKind::Hardware,
            source,
        };
        let webauthn_key = WebAuthn::new(user_id, name, &Passkey::from(attested), authenticator)
            .map_err(|err| TokenError::WebauthnRegistration(err.to_string()))?
            .save(pool)
            .await
            .map_err(|err| TokenError::WebauthnRegistration(err.to_string()))?;
        Ok(webauthn_key)
    }
}
