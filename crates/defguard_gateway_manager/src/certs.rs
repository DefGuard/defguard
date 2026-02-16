//! Cached certificate serials for gateways.

use std::{collections::HashMap, sync::Arc};

use defguard_common::db::{Id, models::gateway::Gateway};
use sqlx::PgPool;
use tokio::sync::watch;

fn collect_certs<I>(items: I) -> HashMap<Id, String>
where
    I: IntoIterator<Item = (Id, Option<String>)>,
{
    items
        .into_iter()
        .filter_map(|(id, cert)| cert.map(|cert| (id, cert)))
        .collect()
}

pub(super) async fn refresh_certs(pool: &PgPool, tx: &watch::Sender<Arc<HashMap<Id, String>>>) {
    match Gateway::all(pool).await {
        Ok(gateways) => {
            let certs = collect_certs(
                gateways
                    .into_iter()
                    .map(|gateway| (gateway.id, gateway.certificate)),
            );
            let _ = tx.send(Arc::new(certs));
        }
        Err(err) => {
            warn!("Failed to refresh gateway certificate list: {err}");
        }
    }
}
