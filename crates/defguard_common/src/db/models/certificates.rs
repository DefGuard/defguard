use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, query, query_as};
use utoipa::ToSchema;

/// Certificate source for the proxy HTTP/HTTPS listener.
///
/// - `None`: no cert configured, proxy runs plain HTTP
/// - `SelfSigned`: cert issued by the Core CA
/// - `LetsEncrypt`: cert obtained via ACME/Let's Encrypt
/// - `Custom`: admin-uploaded PEM cert + key
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, ToSchema, sqlx::Type,
)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum ProxyCertSource {
    #[default]
    None,
    SelfSigned,
    LetsEncrypt,
    Custom,
}

/// Singleton certificates table (id = 1, only one row ever exists).
///
/// Holds the Core CA (used to sign gRPC TLS certs for gateways/proxies)
/// and the proxy HTTP/HTTPS cert (self-signed, Let's Encrypt, or custom).
#[derive(Clone, Debug, Default)]
pub struct Certificates {
    // Core CA
    pub ca_cert_der: Option<Vec<u8>>,
    pub ca_key_der: Option<Vec<u8>>,
    pub ca_expiry: Option<NaiveDateTime>,
    // Proxy HTTP/HTTPS certificate
    pub proxy_http_cert_source: ProxyCertSource,
    pub proxy_http_cert_pem: Option<String>,
    pub proxy_http_cert_key_pem: Option<String>,
    pub proxy_http_cert_expiry: Option<NaiveDateTime>,
    // ACME / Let's Encrypt state (only set when source = LetsEncrypt)
    pub acme_domain: Option<String>,
    /// JSON-serialized instant-acme AccountCredentials.
    pub acme_account_credentials: Option<String>,
}

impl Certificates {
    /// Fetch the singleton row. Returns None if not yet seeded.
    pub async fn get<'e, E>(executor: E) -> sqlx::Result<Option<Self>>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT \
                ca_cert_der, \
                ca_key_der, \
                ca_expiry, \
                proxy_http_cert_source AS \"proxy_http_cert_source: ProxyCertSource\", \
                proxy_http_cert_pem, \
                proxy_http_cert_key_pem, \
                proxy_http_cert_expiry, \
                acme_domain, \
                acme_account_credentials \
            FROM certificates WHERE id = 1"
        )
        .fetch_optional(executor)
        .await
    }

    /// Upsert the singleton row.
    pub async fn save<'e, E>(&self, executor: E) -> sqlx::Result<()>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "INSERT INTO certificates ( \
                id, \
                ca_cert_der, \
                ca_key_der, \
                ca_expiry, \
                proxy_http_cert_source, \
                proxy_http_cert_pem, \
                proxy_http_cert_key_pem, \
                proxy_http_cert_expiry, \
                acme_domain, \
                acme_account_credentials \
            ) VALUES (1, $1, $2, $3, $4, $5, $6, $7, $8, $9) \
            ON CONFLICT (id) DO UPDATE SET \
                ca_cert_der              = EXCLUDED.ca_cert_der, \
                ca_key_der               = EXCLUDED.ca_key_der, \
                ca_expiry                = EXCLUDED.ca_expiry, \
                proxy_http_cert_source   = EXCLUDED.proxy_http_cert_source, \
                proxy_http_cert_pem      = EXCLUDED.proxy_http_cert_pem, \
                proxy_http_cert_key_pem  = EXCLUDED.proxy_http_cert_key_pem, \
                proxy_http_cert_expiry   = EXCLUDED.proxy_http_cert_expiry, \
                acme_domain              = EXCLUDED.acme_domain, \
                acme_account_credentials = EXCLUDED.acme_account_credentials",
            &self.ca_cert_der as &Option<Vec<u8>>,
            &self.ca_key_der as &Option<Vec<u8>>,
            &self.ca_expiry as &Option<NaiveDateTime>,
            &self.proxy_http_cert_source as &ProxyCertSource,
            self.proxy_http_cert_pem,
            self.proxy_http_cert_key_pem,
            &self.proxy_http_cert_expiry as &Option<NaiveDateTime>,
            self.acme_domain,
            self.acme_account_credentials,
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    /// Returns get() result, falling back to a default if the row is missing.
    pub async fn get_or_default<'e, E>(executor: E) -> sqlx::Result<Self>
    where
        E: PgExecutor<'e>,
    {
        Ok(Self::get(executor).await?.unwrap_or_default())
    }
}

/// Returns (cert_pem, key_pem) if a cert is configured, None if the proxy runs plain HTTP.
impl Certificates {
    #[must_use]
    pub fn proxy_http_cert_pair(&self) -> Option<(&str, &str)> {
        match self.proxy_http_cert_source {
            ProxyCertSource::None => None,
            ProxyCertSource::SelfSigned
            | ProxyCertSource::LetsEncrypt
            | ProxyCertSource::Custom => self
                .proxy_http_cert_pem
                .as_deref()
                .zip(self.proxy_http_cert_key_pem.as_deref()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_http_cert_pair() {
        let mut c = Certificates {
            proxy_http_cert_source: ProxyCertSource::None,
            proxy_http_cert_pem: Some("cert".to_string()),
            proxy_http_cert_key_pem: Some("key".to_string()),
            ..Default::default()
        };

        // None always returns None even with PEM fields set
        assert!(c.proxy_http_cert_pair().is_none());

        // All active sources return Some when both fields are present
        for source in [
            ProxyCertSource::SelfSigned,
            ProxyCertSource::LetsEncrypt,
            ProxyCertSource::Custom,
        ] {
            c.proxy_http_cert_source = source;
            assert_eq!(c.proxy_http_cert_pair(), Some(("cert", "key")));
        }

        // Missing either field returns None
        c.proxy_http_cert_source = ProxyCertSource::SelfSigned;
        c.proxy_http_cert_pem = None;
        assert!(c.proxy_http_cert_pair().is_none());

        c.proxy_http_cert_pem = Some("cert".to_string());
        c.proxy_http_cert_key_pem = None;
        assert!(c.proxy_http_cert_pair().is_none());
    }
}
