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

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use defguard_common::db::{
        Id,
        models::{
            gateway::Gateway,
            settings::initialize_current_settings,
            wireguard::WireguardNetwork,
        },
        setup_pool,
    };
    use sqlx::{
        PgPool,
        postgres::{PgConnectOptions, PgPoolOptions},
    };
    use tokio::sync::watch;

    use super::{collect_certs, refresh_certs};

    #[test]
    fn collect_certs_filters_out_gateways_without_certificates() {
        let certs = collect_certs([
            (1, Some("cert-1".to_string())),
            (2, None),
            (3, Some("cert-3".to_string())),
        ]);

        assert_eq!(
            certs,
            HashMap::from([(1, "cert-1".to_string()), (3, "cert-3".to_string())])
        );
    }

    #[sqlx::test]
    async fn refresh_certs_publishes_current_cert_map_to_watch_channel(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to initialize settings for cert refresh tests");

        let network = WireguardNetwork::default()
            .save(&pool)
            .await
            .expect("failed to create network for cert refresh tests");
        let gateway_with_cert =
            create_gateway(&pool, network.id, "gateway-with-cert", Some("cert-1")).await;
        let gateway_without_cert =
            create_gateway(&pool, network.id, "gateway-without-cert", None).await;
        let gateway_with_new_cert =
            create_gateway(&pool, network.id, "gateway-with-new-cert", Some("cert-3")).await;

        let (tx, mut rx) =
            watch::channel(Arc::new(HashMap::from([(999, "stale-cert".to_string())])));

        refresh_certs(&pool, &tx).await;

        assert!(rx.has_changed().expect("cert watch sender should still be alive"));

        let published = Arc::clone(&rx.borrow_and_update());
        let expected = HashMap::from([
            (gateway_with_cert.id, "cert-1".to_string()),
            (gateway_with_new_cert.id, "cert-3".to_string()),
        ]);

        assert_eq!(published.as_ref(), &expected);
        assert!(!published.contains_key(&gateway_without_cert.id));
        assert!(!published.contains_key(&999));
    }

    async fn create_gateway(
        pool: &PgPool,
        location_id: Id,
        name: &str,
        certificate: Option<&str>,
    ) -> Gateway<Id> {
        let mut gateway = Gateway::new(
            location_id,
            name.to_string(),
            "127.0.0.1".to_string(),
            51820,
            "test-admin".to_string(),
        );
        gateway.certificate = certificate.map(str::to_owned);

        gateway
            .save(pool)
            .await
            .expect("failed to create gateway for cert refresh tests")
    }
}
