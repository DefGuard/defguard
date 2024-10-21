use model_derive::Model;
use sqlx::{Error as SqlxError, PgPool};

use crate::db::{Id, NoId};

#[derive(Clone, Debug, Model, PartialEq, Serialize)]
pub struct Gateway<I = NoId> {
    pub id: I,
    pub network_id: Id,
    pub url: String,
}

impl Gateway {
    #[must_use]
    pub fn new<S: Into<String>>(network_id: Id, url: S) -> Self {
        Self {
            id: NoId,
            network_id,
            url: url.into(),
        }
    }
}

impl Gateway<Id> {
    pub async fn find_by_network_id(pool: &PgPool, network_id: Id) -> Result<Vec<Self>, SqlxError> {
        sqlx::query_as!(
            Self,
            "SELECT * FROM gateway WHERE network_id = $1",
            network_id
        )
        .fetch_all(pool)
        .await
    }
}
