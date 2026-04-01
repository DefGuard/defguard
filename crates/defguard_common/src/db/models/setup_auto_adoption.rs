use std::collections::HashMap;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, query, query_scalar, types::Json};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SetupAutoAdoptionComponent {
    Edge,
    Gateway,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoAdoptionWizardStep {
    #[default]
    Welcome,
    AdminUser,
    /// Internal URL settings step (defguard URL + SSL type). Replaces the old UrlSettings step.
    UrlSettings,
    /// SSL configuration result step for the internal URL.
    InternalUrlSslConfig,
    /// External (proxy) URL settings step.
    ExternalUrlSettings,
    /// SSL configuration result step for the external (proxy) URL.
    ExternalUrlSslConfig,
    VpnSettings,
    MfaSettings,
    Summary,
    Finished,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AutoAdoptionComponentResult {
    pub success: bool,
    pub logs: Vec<String>,
    pub updated_at: NaiveDateTime,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AutoAdoptionWizardState {
    #[serde(default)]
    pub step: AutoAdoptionWizardStep,
    #[serde(default)]
    pub adoption_result: HashMap<SetupAutoAdoptionComponent, AutoAdoptionComponentResult>,
}

impl AutoAdoptionWizardState {
    pub async fn save<'e, E>(&self, executor: E) -> sqlx::Result<()>
    where
        E: PgExecutor<'e>,
    {
        let auto_adoption_state =
            serde_json::to_value(self).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

        query(
            "UPDATE wizard SET auto_adoption_state = $1
             WHERE is_singleton",
        )
        .bind(auto_adoption_state)
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn get<'e, E>(executor: E) -> sqlx::Result<Option<Self>>
    where
        E: PgExecutor<'e>,
    {
        let state: Option<Json<Self>> = query_scalar(
            "SELECT auto_adoption_state
             FROM wizard
             WHERE is_singleton
             LIMIT 1",
        )
        .fetch_one(executor)
        .await?;

        Ok(state.map(|j| j.0))
    }

    pub async fn clear<'e, E>(executor: E) -> sqlx::Result<()>
    where
        E: PgExecutor<'e>,
    {
        query(
            "UPDATE wizard
             SET auto_adoption_state = NULL
             WHERE is_singleton",
        )
        .execute(executor)
        .await?;

        Ok(())
    }
}
