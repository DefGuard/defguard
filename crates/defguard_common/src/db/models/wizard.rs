use std::fmt;

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgExecutor, Type};
use tracing::info;

use super::setup_auto_adoption::AutoAdoptionWizardStep;
use crate::db::models::{
    InitialSetupState, InitialSetupStep, setup_auto_adoption::AutoAdoptionWizardState,
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

/// The wizard singleton row.
///
/// `active_wizard` and `completed` are regular DB columns.
/// Each wizard type has its own JSONB column for step-tracking state.
#[derive(Debug, Serialize)]
pub struct Wizard {
    pub active_wizard: ActiveWizard,
    pub completed: bool,
}

#[derive(Debug, FromRow)]
struct WizardDbRow {
    active_wizard: ActiveWizard,
    completed: bool,
}

impl Wizard {
    pub async fn save<'e, E>(&self, executor: E) -> Result<(), sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        sqlx::query(
            "UPDATE wizard SET active_wizard = $1, completed = $2 \
			 WHERE is_singleton",
        )
        .bind(self.active_wizard)
        .bind(self.completed)
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn get<'e, E>(executor: E) -> Result<Self, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        let row = sqlx::query_as::<_, WizardDbRow>(
            "SELECT active_wizard, completed \
			 FROM wizard \
			 WHERE is_singleton \
			 LIMIT 1",
        )
        .fetch_one(executor)
        .await?;

        Ok(Self {
            active_wizard: row.active_wizard,
            completed: row.completed,
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
    pub async fn requires_auth<'e, E>(&self, executor: E) -> Result<bool, sqlx::Error>
    where
        E: PgExecutor<'e> + Copy,
    {
        match self.active_wizard {
            ActiveWizard::Initial => {
                let state = InitialSetupState::get(executor).await?.unwrap_or_default();
                let step = state.step;
                Ok(step > InitialSetupStep::AdminUser)
            }
            ActiveWizard::AutoAdoption => {
                let state = AutoAdoptionWizardState::get(executor)
                    .await?
                    .unwrap_or_default();
                let step = state.step;
                Ok(step > AutoAdoptionWizardStep::AdminUser)
            }
            _ => Ok(true),
        }
    }
}
