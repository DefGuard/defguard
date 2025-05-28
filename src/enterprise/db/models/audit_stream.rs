use model_derive::Model;
use pgp::packet::config;
use serde::Serialize;
use sqlx::{query_as, Error as SqlxError, FromRow, PgExecutor, Type};
use strum_macros::{Display, EnumString};

use crate::{
    db::{Id, NoId},
    enterprise::audit_stream::error::AuditStreamError,
    secret::SecretStringWrapper,
};

#[derive(Debug, Serialize, Deserialize, Type, EnumString, Display, Clone)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AuditStreamType {
    #[strum(serialize = "vector_http")]
    VectorHttp,
}

#[derive(Debug, Serialize, Model, FromRow)]
#[table(audit_stream)]
pub struct AuditStreamModel<I = NoId> {
    pub id: I,
    #[model(enum)]
    pub stream_type: AuditStreamType,
    pub config: serde_json::Value,
}

pub enum AuditStreamConfig {
    VectorHttp(VectorHttpAuditStream),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VectorHttpAuditStream {
    pub url: String,
    pub username: String,
    pub password: SecretStringWrapper,
}

impl AuditStreamConfig {
    pub fn from(model: AuditStreamModel<Id>) -> Result<Self, AuditStreamError> {
        match model.stream_type {
            AuditStreamType::VectorHttp => {
                match serde_json::from_value::<VectorHttpAuditStream>(model.config) {
                    Ok(deserialized) => Ok(Self::VectorHttp(deserialized)),
                    Err(e) => Err(AuditStreamError::ConfigDeserializeError(
                        model.stream_type.to_string(),
                        e.to_string(),
                    )),
                }
            }
        }
    }
}

impl AuditStreamModel<Id> {
    pub async fn find_by_stream_type<'e, E>(
        executor: E,
        stream_type: &AuditStreamType,
    ) -> Result<Vec<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let configs = query_as!(
            AuditStreamModel,
            "SELECT id, stream_type \"stream_type: AuditStreamType\", config \
            FROM audit_stream \
            WHERE stream_type = $1",
            stream_type.to_string()
        )
        .fetch_all(executor)
        .await?;
        Ok(configs)
    }
}
