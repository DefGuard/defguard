use std::fmt::{self, Display, Formatter};

use chrono::{NaiveDateTime, Utc};
use model_derive::Model;
use sqlx::{query_as, Error as SqlxError};

use crate::db::DbPool;

#[derive(Clone, Deserialize, Model, Serialize, Debug)]
#[table(device_login_event)]
pub struct DeviceLoginEvent {
    id: Option<i64>,
    pub user_id: i64,
    pub ip_address: String,
    pub model: Option<String>,
    pub family: String,
    pub brand: Option<String>,
    pub os_family: String,
    pub browser: String,
    pub event_type: String,
    pub created: NaiveDateTime,
}

impl Display for DeviceLoginEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.id {
            Some(device_id) => write!(f, "[ID {}] {}", device_id, self.family),
            None => write!(f, "{}", self.family),
        }
    }
}

impl DeviceLoginEvent {
    #[must_use]
    pub fn new(
        user_id: i64,
        ip_address: String,
        model: Option<String>,
        family: String,
        brand: Option<String>,
        os_family: String,
        browser: String,
        event_type: String,
    ) -> Self {
        Self {
            id: None,
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

    pub async fn find_device_login_event(&self, pool: &DbPool) -> Result<Option<Self>, SqlxError> {
        query_as!(
          Self,
          "SELECT id \"id?\", user_id, ip_address, model, family, brand, os_family, browser, event_type, created
          FROM device_login_event WHERE user_id = $1 AND event_type = $2 AND family = $3 AND \
          brand = $4 AND model = $5 AND browser = $6",
          self.user_id, self.event_type, self.family, self.brand, self.model, self.browser
      )
        .fetch_optional(pool)
        .await
    }

    pub async fn check_if_device_already_logged_in(
        mut self,
        pool: &DbPool,
    ) -> Result<Option<Self>, anyhow::Error> {
        let existing_login_event = self.find_device_login_event(pool).await?;

        if existing_login_event.is_none() {
            self.save(pool).await?;
            Ok(Some(self))
        } else {
            Ok(None)
        }
    }
}
