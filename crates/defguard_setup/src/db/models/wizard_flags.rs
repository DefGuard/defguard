use sqlx::{PgExecutor, prelude::FromRow};

#[derive(Debug, FromRow)]
pub struct WizardFlags {
    pub migration_wizard_needed: bool,
    pub migration_wizard_in_progress: bool,
    pub migration_wizard_completed: bool,
    pub initial_wizard_completed: bool,
    pub initial_wizard_in_progress: bool,
}

impl WizardFlags {
    pub async fn save<'e, E>(&self, executor: E) -> Result<(), sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        sqlx::query(
            "UPDATE wizard
             SET
                migration_wizard_needed = $1,
                     migration_wizard_in_progress = $2,
                     migration_wizard_completed = $3,
                     initial_wizard_in_progress = $4,
                     initial_wizard_completed = $5
             WHERE is_singleton = TRUE",
        )
        .bind(self.migration_wizard_needed)
        .bind(self.migration_wizard_in_progress)
        .bind(self.migration_wizard_completed)
        .bind(self.initial_wizard_in_progress)
        .bind(self.initial_wizard_completed)
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn get<'e, E>(executor: E) -> Result<Self, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        sqlx::query_as!(
            Self,
            "SELECT
                migration_wizard_needed,
                migration_wizard_in_progress,
                migration_wizard_completed,
                initial_wizard_in_progress,
                initial_wizard_completed
             FROM wizard
             LIMIT 1"
        )
        .fetch_one(executor)
        .await
    }

    pub async fn init<'e, E>(executor: E) -> Result<Self, sqlx::Error>
    where
        E: PgExecutor<'e> + Copy,
    {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM wizard")
            .fetch_one(executor)
            .await?;

        if count == 0 {
            let is_fresh_instance: bool = sqlx::query_scalar(
                "SELECT
                    (SELECT COUNT(*) FROM \"user\") = 0
                    AND (SELECT COUNT(*) FROM wireguard_network) = 0
                    AND (SELECT COUNT(*) FROM \"device\") = 0",
            )
            .fetch_one(executor)
            .await?;

            sqlx::query(
                "INSERT INTO wizard (
                    migration_wizard_needed,
                    migration_wizard_in_progress,
                    migration_wizard_completed,
                    initial_wizard_in_progress,
                    initial_wizard_completed
                ) VALUES (FALSE, FALSE, FALSE, $1, FALSE)",
            )
            .bind(is_fresh_instance)
            .execute(executor)
            .await?;

            return Ok(Self {
                migration_wizard_needed: false,
                migration_wizard_in_progress: false,
                migration_wizard_completed: false,
                initial_wizard_in_progress: is_fresh_instance,
                initial_wizard_completed: false,
            });
        }
        Self::get(executor).await
    }
}
