use std::fmt;

use chrono::{NaiveDateTime, Utc};
use model_derive::Model;
use sqlx::{query_as, Error as SqlxError, PgPool};

use crate::db::{Id, NoId};

#[derive(Clone, Deserialize, Model, Serialize, Debug)]
#[table(device_login_event)]
pub struct DeviceLoginEvent<I = NoId> {
    id: I,
    pub user_id: Id,
    pub ip_address: String,
    pub model: Option<String>,
    pub family: String,
    pub brand: Option<String>,
    pub os_family: String,
    pub browser: String,
    pub event_type: String,
    pub created: NaiveDateTime,
}

impl fmt::Display for DeviceLoginEvent<NoId> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.family)
    }
}

impl fmt::Display for DeviceLoginEvent<Id> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[ID {}] {}", self.id, self.family)
    }
}

impl DeviceLoginEvent {
    #[must_use]
    pub fn new(
        user_id: Id,
        ip_address: String,
        model: Option<String>,
        family: String,
        brand: Option<String>,
        os_family: String,
        browser: String,
        event_type: String,
    ) -> Self {
        Self {
            id: NoId,
            user_id,
            ip_address,
            model,
            family,
            brand,
            os_family,
            browser,
            event_type,
            created: Utc::now().naive_utc(),
        }
    }

    pub(crate) async fn check_if_device_already_logged_in(
        self,
        pool: &PgPool,
    ) -> Result<Option<DeviceLoginEvent<Id>>, anyhow::Error> {
        let existing_login_event = self.find_device_login_event(pool).await?;

        if existing_login_event.is_none() {
            Ok(Some(self.save(pool).await?))
        } else {
            Ok(None)
        }
    }

    pub async fn find_device_login_event(
        &self,
        pool: &PgPool,
    ) -> Result<Option<DeviceLoginEvent<Id>>, SqlxError> {
        query_as!(
            DeviceLoginEvent::<Id>,
            "SELECT id, user_id, ip_address, model, family, brand, os_family, browser, event_type, created \
            FROM device_login_event WHERE user_id = $1 AND event_type = $2 AND family = $3 AND \
            brand = $4 AND model = $5 AND browser = $6",
            self.user_id, self.event_type, self.family, self.brand, self.model, self.browser
        )
        .fetch_optional(pool)
        .await
    }
}
