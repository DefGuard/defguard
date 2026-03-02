use std::collections::{BTreeMap, HashMap};

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, types::Json};

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
    UrlSettings,
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
    fn new() -> Self {
        Self {
            step: AutoAdoptionWizardStep::default(),
            adoption_result: HashMap::new(),
        }
    }

    pub async fn load(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let state_json = sqlx::query_scalar::<_, Json<AutoAdoptionWizardState>>(
            "SELECT auto_adoption_wizard_state FROM wizard WHERE is_singleton = TRUE",
        )
        .fetch_one(pool)
        .await?;

        Ok(state_json.0)
    }

    pub async fn save(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE wizard SET auto_adoption_wizard_state = $1 WHERE is_singleton = TRUE")
            .bind(Json(self))
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn insert_component_result(
        &mut self,
        pool: &PgPool,
        component: SetupAutoAdoptionComponent,
        result: AutoAdoptionComponentResult,
    ) -> Result<(), sqlx::Error> {
        self.adoption_result.insert(component, result);
        self.save(pool).await
    }
}
