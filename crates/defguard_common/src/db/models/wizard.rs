use std::fmt;

use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, Type, types::Json};
use tracing::info;

use super::{
    migration_wizard::MigrationWizardState,
    settings::InitialSetupStep,
    setup_auto_adoption::{AutoAdoptionWizardState, AutoAdoptionWizardStep},
};

/// Which wizard is currently active. Stored as a PostgreSQL enum column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "active_wizard", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ActiveWizard {
    None,
    Initial,
    AutoAdoption,
    Migration,
}

impl fmt::Display for ActiveWizard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Initial => write!(f, "initial setup"),
            Self::AutoAdoption => write!(f, "auto-adoption"),
            Self::Migration => write!(f, "migration"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitialSetupState {
    pub step: InitialSetupStep,
}

/// The wizard singleton row.
///
/// `active_wizard` and `completed` are regular DB columns.
/// Each wizard type has its own JSONB column for step-tracking state.
#[derive(Debug, Serialize)]
pub struct Wizard {
    pub active_wizard: ActiveWizard,
    pub completed: bool,
    pub initial_setup_state: Option<InitialSetupState>,
    pub auto_adoption_state: Option<AutoAdoptionWizardState>,
    pub migration_wizard_state: Option<MigrationWizardState>,
}

/// Internal row type used for SQLx deserialization.
struct WizardRow {
    active_wizard: ActiveWizard,
    completed: bool,
    initial_setup_state: Option<Json<InitialSetupState>>,
    auto_adoption_state: Option<Json<AutoAdoptionWizardState>>,
    migration_wizard_state: Option<Json<MigrationWizardState>>,
}

impl Wizard {
    pub async fn save<'e, E>(&self, executor: E) -> Result<(), sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        let initial_setup_state = self
            .initial_setup_state
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|err| sqlx::Error::Decode(Box::new(err)))?;
        let auto_adoption_state = self
            .auto_adoption_state
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|err| sqlx::Error::Decode(Box::new(err)))?;
        let migration_wizard_state = self
            .migration_wizard_state
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

        sqlx::query(
            "UPDATE wizard SET active_wizard = $1, completed = $2, \
			 initial_setup_state = $3, auto_adoption_state = $4, \
			 migration_wizard_state = $5 \
			 WHERE is_singleton = TRUE",
        )
        .bind(self.active_wizard)
        .bind(self.completed)
        .bind(initial_setup_state)
        .bind(auto_adoption_state)
        .bind(migration_wizard_state)
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn get<'e, E>(executor: E) -> Result<Self, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        let row = sqlx::query_as!(
            WizardRow,
            "SELECT active_wizard AS \"active_wizard!: ActiveWizard\", \
			        completed, \
			        initial_setup_state AS \"initial_setup_state: Json<InitialSetupState>\", \
			        auto_adoption_state AS \"auto_adoption_state: Json<AutoAdoptionWizardState>\", \
			        migration_wizard_state AS \"migration_wizard_state: Json<MigrationWizardState>\" \
			 FROM wizard \
			 WHERE is_singleton = TRUE \
			 LIMIT 1"
        )
        .fetch_one(executor)
        .await?;

        Ok(Self {
            active_wizard: row.active_wizard,
            completed: row.completed,
            initial_setup_state: row.initial_setup_state.map(|j| j.0),
            auto_adoption_state: row.auto_adoption_state.map(|j| j.0),
            migration_wizard_state: row.migration_wizard_state.map(|j| j.0),
        })
    }

    /// Initialize the wizard at startup.
    ///
    /// The wizard row is always seeded by the migration. If `active_wizard`
    /// is still `None` (i.e. no wizard has been activated yet), detect which
    /// one should be active based on database state:
    /// - Existing data (users/networks/devices) = `Migration`
    /// - Fresh install with auto-adoption CLI flags = `AutoAdoption`
    /// - Fresh install without flags = `Initial`
    pub async fn init<'e, E>(executor: E, has_auto_adopt_flags: bool) -> Result<Self, sqlx::Error>
    where
        E: PgExecutor<'e> + Copy,
    {
        let mut wizard = Self::get(executor).await?;

        if wizard.completed {
            info!("Wizard already completed, skipping initialization");
            return Ok(wizard);
        }

        if wizard.active_wizard != ActiveWizard::None {
            info!("Resuming {} wizard", wizard.active_wizard);
            return Ok(wizard);
        }

        let is_fresh_instance: bool = sqlx::query_scalar(
            "SELECT
				(SELECT COUNT(*) FROM \"user\") = 0
				AND (SELECT COUNT(*) FROM wireguard_network) = 0
				AND (SELECT COUNT(*) FROM \"device\") = 0",
        )
        .fetch_one(executor)
        .await?;

        let active_wizard;

        if has_auto_adopt_flags {
            active_wizard = ActiveWizard::AutoAdoption;
        } else if is_fresh_instance {
            active_wizard = ActiveWizard::Initial;
        } else {
            active_wizard = ActiveWizard::Migration;
        }

        wizard.active_wizard = active_wizard;

        info!("Starting {active_wizard} wizard");

        wizard.save(executor).await?;

        Ok(wizard)
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active_wizard != ActiveWizard::None
    }

    /// Returns `true` when the current wizard state requires authentication.
    ///
    /// During the Initial and AutoAdoption wizards, unauthenticated access is
    /// allowed until the admin user has been created (i.e. the wizard step is
    /// at or before `AdminUser`). All other wizard types (or steps past admin
    /// creation) require a valid session.
    #[must_use]
    pub fn requires_auth(&self) -> bool {
        match self.active_wizard {
            ActiveWizard::Initial => {
                let step = self
                    .initial_setup_state
                    .as_ref()
                    .map(|s| s.step)
                    .unwrap_or(InitialSetupStep::Welcome);
                step > InitialSetupStep::AdminUser
            }
            ActiveWizard::AutoAdoption => {
                let step = self
                    .auto_adoption_state
                    .as_ref()
                    .map(|s| s.step)
                    .unwrap_or(AutoAdoptionWizardStep::Welcome);
                step > AutoAdoptionWizardStep::AdminUser
            }
            _ => true,
        }
    }
}
