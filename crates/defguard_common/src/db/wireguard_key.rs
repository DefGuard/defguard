use base64::prelude::{BASE64_STANDARD, Engine};
use x25519_dalek::{PublicKey, StaticSecret};

/// Utility structure for WireGuard keypair.
pub struct WireguardKey {
    private: StaticSecret,
    public: PublicKey,
}

impl WireguardKey {
    /// Generate random keypair.
    #[must_use]
    pub fn generate() -> Self {
        let private = StaticSecret::random();
        let public = PublicKey::from(&private);
        Self { private, public }
    }

    /// Return private key as base64-encoded string.
    #[must_use]
    pub fn private(&self) -> String {
        BASE64_STANDARD.encode(self.private.to_bytes())
    }

    /// Return public key as base64-encoded string.
    #[must_use]
    pub fn public(&self) -> String {
        BASE64_STANDARD.encode(self.public.to_bytes())
    }
}
