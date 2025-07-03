use rand::{Rng, distributions::Alphanumeric, thread_rng};

/// Generate random alphanumeric string.
#[must_use]
pub(crate) fn gen_alphanumeric(n: usize) -> String {
    thread_rng()
        .sample_iter(Alphanumeric)
        .take(n)
        .map(char::from)
        .collect()
}

/// Generate random 20-byte secret for TOTP.
#[must_use]
pub(crate) fn gen_totp_secret() -> Vec<u8> {
    thread_rng().r#gen::<[u8; 20]>().to_vec()
}
