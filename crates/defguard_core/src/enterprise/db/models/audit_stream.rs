use model_derive::Model;
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
    #[strum(serialize = "logstash_http")]
    LogstashHttp,
}

#[derive(Debug, Serialize, Model, FromRow)]
#[table(audit_stream)]
pub struct AuditStream<I = NoId> {
    pub id: I,
    pub name: Option<String>,
    #[model(enum)]
    pub stream_type: AuditStreamType,
    pub config: serde_json::Value,
}

#[derive(Debug)]
pub enum AuditStreamConfig {
    VectorHttp(VectorHttpAuditStream),
    LogstashHttp(LogstashHttpAuditStream),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogstashHttpAuditStream {
    pub url: String,
    // cert to use for tls
    pub cert: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VectorHttpAuditStream {
    pub url: String,
    pub username: Option<String>,
    pub password: Option<SecretStringWrapper>,
    // cert to use for tls
    pub cert: Option<String>,
}

impl AuditStreamConfig {
    pub fn from_serde_value(
        stream_type: &AuditStreamType,
        value: &serde_json::Value,
    ) -> Result<Self, AuditStreamError> {
        match stream_type {
            AuditStreamType::VectorHttp => {
                match serde_json::from_value::<VectorHttpAuditStream>(value.clone()) {
                    Ok(deserialized) => Ok(Self::VectorHttp(deserialized)),
                    Err(e) => Err(AuditStreamError::ConfigDeserializeError(
                        stream_type.to_string(),
                        e.to_string(),
                    )),
                }
            }
            AuditStreamType::LogstashHttp => {
                match serde_json::from_value::<LogstashHttpAuditStream>(value.clone()) {
                    Ok(deserialized) => Ok(Self::LogstashHttp(deserialized)),
                    Err(e) => Err(AuditStreamError::ConfigDeserializeError(
                        stream_type.to_string(),
                        e.to_string(),
                    )),
                }
            }
        }
    }

    pub fn from(model: &AuditStream<Id>) -> Result<Self, AuditStreamError> {
        Self::from_serde_value(&model.stream_type, &model.config)
    }
}

impl AuditStream<Id> {
    pub async fn find_by_stream_type<'e, E>(
        executor: E,
        stream_type: &AuditStreamType,
    ) -> Result<Vec<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let configs: Vec<AuditStream<Id>> = query_as!(
            AuditStream,
            "SELECT id, name, stream_type \"stream_type: AuditStreamType\", config \
            FROM audit_stream \
            WHERE stream_type = $1",
            stream_type.to_string()
        )
        .fetch_all(executor)
        .await?;
        Ok(configs)
    }
}
