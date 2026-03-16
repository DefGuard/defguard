use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;

#[derive(Debug, Serialize, Deserialize, Default)]
pub enum MigrationWizardStep {
    #[default]
    #[serde(rename = "welcome")]
    Welcome,
    #[serde(rename = "general")]
    General,
    #[serde(rename = "ca")]
    Ca,
    #[serde(rename = "caSummary")]
    CaSummary,
    #[serde(rename = "edgeDeployment")]
    EdgeDeployment,
    #[serde(rename = "edge")]
    Edge,
    #[serde(rename = "edgeAdoption")]
    EdgeAdoption,
    #[serde(rename = "confirmation")]
    Confirmation,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MigrationWizardLocationState {
    pub(crate) locations: Vec<i64>,
    pub(crate) current_location: i64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct MigrationWizardState {
    pub current_step: MigrationWizardStep,
    pub location_state: Option<MigrationWizardLocationState>,
}

impl MigrationWizardState {
    pub async fn get<'e, E>(executor: E) -> Result<Option<Self>, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        let state: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT migration_wizard_state
             FROM wizard
             LIMIT 1",
        )
        .fetch_optional(executor)
        .await?
        .flatten();

        state
            .map(serde_json::from_value)
            .transpose()
            .map_err(|error| sqlx::Error::Decode(Box::new(error)))
    }

    pub async fn save<'e, E>(&self, executor: E) -> Result<(), sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        let state =
            serde_json::to_value(self).map_err(|error| sqlx::Error::Decode(Box::new(error)))?;

        sqlx::query(
            "UPDATE wizard
             SET migration_wizard_state = $1
             WHERE is_singleton",
        )
        .bind(state)
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn clear<'e, E>(executor: E) -> Result<(), sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        sqlx::query!(
            "Update wizard \
            SET migration_wizard_state = NULL \
            WHERE is_singleton"
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}
