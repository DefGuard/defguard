pub mod db;
pub mod random;
pub mod secret;

pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "+", env!("VERGEN_GIT_SHA"));
pub const CARGO_VERSION: &str = env!("CARGO_PKG_VERSION");
