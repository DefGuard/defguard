use chrono::NaiveDateTime;
use model_derive::Model;
use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::db::{Id, NoId};

#[derive(Model, Serialize, ToSchema)]
pub struct Proxy<I = NoId> {
    pub id: I,
    pub name: String,
    pub address: String,
    pub port: i32,
    pub public_address: String,
    pub connected_at: Option<NaiveDateTime>,
    pub disconnected_at: Option<NaiveDateTime>,
    pub version: Option<String>,
    pub has_certificate: bool,
    pub certificate_expiry: Option<NaiveDateTime>,
}

impl Proxy {
    pub fn new<S: Into<String>>(name: S, address: S, port: i32, public_address: S) -> Self {
        Self {
            id: NoId,
            name: name.into(),
            address: address.into(),
            port,
            public_address: public_address.into(),
            connected_at: None,
            disconnected_at: None,
            has_certificate: false,
            certificate_expiry: None,
            version: None,
        }
    }
}

impl Proxy<Id> {
    pub async fn find_by_address_port(
        pool: &PgPool,
        address: &str,
        port: i32,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Proxy,
            "SELECT * FROM proxy WHERE address = $1 AND port = $2",
            address,
            port
        )
        .fetch_optional(pool)
        .await
    }
}
