use chrono::NaiveDateTime;
use model_derive::Model;
use sqlx::{query_as, PgExecutor};

use crate::db::{Id, NoId};

#[derive(Clone, Debug, Model, PartialEq, Serialize)]
pub struct Gateway<I = NoId> {
    pub id: I,
    pub network_id: Id,
    pub url: String,
    pub connected: bool,
    pub hostname: Option<String>,
    pub connected_at: Option<NaiveDateTime>,
    pub disconnected_at: Option<NaiveDateTime>,
}

impl Gateway {
    #[must_use]
    pub(crate) fn new<S: Into<String>>(network_id: Id, url: S) -> Self {
        Self {
            id: NoId,
            network_id,
            url: url.into(),
            connected: false,
            hostname: None,
            connected_at: None,
            disconnected_at: None,
        }
    }
}

impl Gateway<Id> {
    pub(crate) async fn find_by_network_id<'e, E>(
        executor: E,
        network_id: Id,
    ) -> Result<Vec<Self>, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT * FROM gateway WHERE network_id = $1 ORDER BY id",
            network_id
        )
        .fetch_all(executor)
        .await
    }
}
