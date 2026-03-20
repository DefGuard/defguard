use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, Type, query, query_scalar, types::Json};

#[derive(Clone, Debug, Copy, Eq, PartialEq, Deserialize, Serialize, Default, Type, PartialOrd)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "initial_setup_step", rename_all = "snake_case")]
pub enum InitialSetupStep {
    #[default]
    Welcome,
    AdminUser,
    GeneralConfiguration,
    Ca,
    CaSummary,
    // Adoption is not present, since the proxy is saved
    // only after completing adoption step.
    EdgeComponent,
    Confirmation,
    Finished,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct InitialSetupState {
    pub step: InitialSetupStep,
}

impl InitialSetupState {
    pub async fn set_step<'e, E>(executor: E, step: InitialSetupStep) -> sqlx::Result<()>
    where
        E: PgExecutor<'e> + Copy,
    {
        let mut state = Self::get(executor).await?.unwrap_or(Self {
            step: InitialSetupStep::Welcome,
        });
        state.step = step;
        state.save(executor).await?;

        Ok(())
    }

    pub async fn save<'e, E>(&self, executor: E) -> sqlx::Result<()>
    where
        E: PgExecutor<'e>,
    {
        let initial_setup_state =
            serde_json::to_value(self).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

        query(
            "UPDATE wizard SET initial_setup_state = $1
             WHERE is_singleton",
        )
        .bind(initial_setup_state)
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn get<'e, E>(executor: E) -> sqlx::Result<Option<Self>>
    where
        E: PgExecutor<'e>,
    {
        let state: Option<Json<Self>> = query_scalar(
            "SELECT initial_setup_state
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
             SET initial_setup_state = NULL
             WHERE is_singleton",
        )
        .execute(executor)
        .await?;

        Ok(())
    }
}
