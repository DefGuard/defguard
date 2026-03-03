use std::collections::HashMap;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

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
