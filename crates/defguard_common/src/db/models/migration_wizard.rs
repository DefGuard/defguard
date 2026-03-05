use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Default)]
pub enum MigrationWizardStep {
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
    pub locations: Vec<i64>,
    pub current_location: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationWizardState {
    pub location_state: Option<MigrationWizardLocationState>,
}
