use chrono::NaiveDateTime;
use model_derive::Model;
use sqlx::{PgExecutor, query};

use crate::db::{Id, NoId};

#[derive(Model)]
#[table(revoked_certificates)]
pub struct RevokedCertificate<I = NoId> {
    pub id: I,
    pub certificate: String,
    pub revoked_at: NaiveDateTime,
    pub certificate_expiry: NaiveDateTime,
}

impl RevokedCertificate {
    pub async fn list<'e, E>(executor: E) -> sqlx::Result<Vec<String>>
    where
        E: PgExecutor<'e>,
    {
        let rows = query!("SELECT certificate FROM revoked_certificates")
            .fetch_all(executor)
            .await?;
        Ok(rows.into_iter().map(|row| row.certificate).collect())
    }
}
