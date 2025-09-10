use model_derive::Model;
use serde::Serialize;
use sqlx::{Error as SqlxError, FromRow, PgExecutor, Type, query_as};
use strum_macros::{Display, EnumString};

use crate::{
    db::{Id, NoId},
    enterprise::activity_log_stream::error::ActivityLogStreamError,
    secret::SecretStringWrapper,
};

#[derive(Debug, Serialize, Deserialize, Type, EnumString, Display, Clone)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ActivityLogStreamType {
    #[strum(serialize = "vector_http")]
    VectorHttp,
    #[strum(serialize = "logstash_http")]
    LogstashHttp,
}

#[derive(Clone, Debug, Serialize, Model, FromRow)]
#[table(activity_log_stream)]
pub struct ActivityLogStream<I = NoId> {
    pub id: I,
    pub name: String,
    #[model(enum)]
    pub stream_type: ActivityLogStreamType,
    pub config: serde_json::Value,
}

#[derive(Debug)]
pub enum ActivityLogStreamConfig {
    VectorHttp(VectorHttpActivityLogStream),
    LogstashHttp(LogstashHttpActivityLogStream),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogstashHttpActivityLogStream {
    pub url: String,
    pub username: Option<String>,
    pub password: Option<SecretStringWrapper>,
    // cert to use for tls
    pub cert: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VectorHttpActivityLogStream {
    pub url: String,
    pub username: Option<String>,
    pub password: Option<SecretStringWrapper>,
    // cert to use for tls
    pub cert: Option<String>,
}

impl ActivityLogStreamConfig {
    pub fn from_serde_value(
        stream_type: &ActivityLogStreamType,
        value: &serde_json::Value,
    ) -> Result<Self, ActivityLogStreamError> {
        match stream_type {
            ActivityLogStreamType::VectorHttp => {
                match serde_json::from_value::<VectorHttpActivityLogStream>(value.clone()) {
                    Ok(deserialized) => Ok(Self::VectorHttp(deserialized)),
                    Err(e) => Err(ActivityLogStreamError::ConfigDeserializeError(
                        stream_type.to_string(),
                        e.to_string(),
                    )),
                }
            }
            ActivityLogStreamType::LogstashHttp => {
                match serde_json::from_value::<LogstashHttpActivityLogStream>(value.clone()) {
                    Ok(deserialized) => Ok(Self::LogstashHttp(deserialized)),
                    Err(e) => Err(ActivityLogStreamError::ConfigDeserializeError(
                        stream_type.to_string(),
                        e.to_string(),
                    )),
                }
            }
        }
    }

    pub fn from(model: &ActivityLogStream<Id>) -> Result<Self, ActivityLogStreamError> {
        Self::from_serde_value(&model.stream_type, &model.config)
    }
}

impl ActivityLogStream<Id> {
    pub async fn find_by_stream_type<'e, E>(
        executor: E,
        stream_type: &ActivityLogStreamType,
    ) -> Result<Vec<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let configs: Vec<ActivityLogStream<Id>> = query_as!(
            ActivityLogStream,
            "SELECT id, name, stream_type \"stream_type: ActivityLogStreamType\", config \
            FROM activity_log_stream \
            WHERE stream_type = $1",
            stream_type.to_string()
        )
        .fetch_all(executor)
        .await?;
        Ok(configs)
    }
}
