use rand::{distributions::Alphanumeric, thread_rng, Rng};

#[must_use]
pub(crate) fn gen_alphanumeric(n: usize) -> String {
    thread_rng()
        .sample_iter(Alphanumeric)
        .take(n)
        .map(char::from)
        .collect()
}
