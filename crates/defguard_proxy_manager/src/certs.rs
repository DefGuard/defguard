//! Cached certificate serials for proxies.

use std::{collections::HashMap, sync::Arc};

use defguard_common::db::{Id, models::proxy::Proxy};
use sqlx::PgPool;
use tokio::sync::watch;

/// Build a compact id->serial map, skipping proxies without a stored cert.
fn collect_certs<I>(items: I) -> HashMap<Id, String>
where
    I: IntoIterator<Item = (Id, Option<String>)>,
{
    items
        .into_iter()
        .filter_map(|(id, cert)| cert.map(|cert| (id, cert)))
        .collect()
}

/// Refresh the cached cert serials for all proxies.
pub(crate) async fn refresh_certs(pool: &PgPool, tx: &watch::Sender<Arc<HashMap<Id, String>>>) {
    match Proxy::all(pool).await {
        Ok(proxies) => {
            let certs = collect_certs(
                proxies
                    .into_iter()
                    .map(|proxy| (proxy.id, proxy.certificate)),
            );
            let _ = tx.send(Arc::new(certs));
        }
        Err(err) => {
            warn!("Failed to refresh revoked certificate list: {err}");
        }
    }
}
