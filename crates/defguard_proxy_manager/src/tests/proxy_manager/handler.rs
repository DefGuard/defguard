#[path = "handler/support.rs"]
mod support;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use self::support::complete_proxy_handshake;
use crate::tests::common::{HandlerTestContext, reload_proxy};

include!("handler/lifecycle.rs");
