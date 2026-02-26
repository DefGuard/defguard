use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;

#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) enum MigrationWizardStep {
    #[default]
    Welcome,
    GeneralConfiguration,
    CertificateAuthority,
    CertificateSummary,
    EdgeComponent,
    EdgeComponentAdaptation,
    Confirmation,
    LocationMigration,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MigrationWizardLocationState {
    pub(crate) locations: Vec<i64>,
    pub(crate) current_location: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationWizardState {
    pub location_state: Option<MigrationWizardLocationState>,
}

impl MigrationWizardState {
    pub(crate) async fn get<'e, E>(executor: E) -> Result<Option<Self>, sqlx::Error>
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

    pub(crate) async fn save<'e, E>(&self, executor: E) -> Result<(), sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        let state =
            serde_json::to_value(self).map_err(|error| sqlx::Error::Decode(Box::new(error)))?;

        sqlx::query(
            "UPDATE wizard
             SET migration_wizard_state = $1
             WHERE is_singleton = TRUE",
        )
        .bind(state)
        .execute(executor)
        .await?;

        Ok(())
    }
}
