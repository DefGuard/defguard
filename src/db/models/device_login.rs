use std::fmt::{Display, Formatter};

use chrono::{NaiveDateTime, Utc};
use model_derive::Model;

use crate::db::DbPool;
use sqlx::{query_as, Error as SqlxError};

#[derive(Clone, Deserialize, Model, Serialize, Debug)]
#[table(device_login_event)]
pub struct DeviceLoginEvent {
    id: Option<i64>,
    pub user_id: i64,
    pub model: Option<String>,
    pub family: String,
    pub brand: Option<String>,
    pub os_family: String,
    pub browser: String,
    pub event_type: String,
    pub created: NaiveDateTime,
}

impl Display for DeviceLoginEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
            model,
            family,
            brand,
            os_family,
            browser,
            event_type,
            created: Utc::now().naive_utc(),
        }
    }

    pub async fn find_device_login_event(
        pool: &DbPool,
        device_login_event: &Self,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
          Self,
          "SELECT id \"id?\", user_id, model, family, brand, os_family, browser, event_type, created
          FROM device_login_event WHERE user_id = $1 AND event_type = $2 AND family = $3",
          device_login_event.user_id, device_login_event.event_type, device_login_event.family
      )
        .fetch_optional(pool)
        .await
    }

    pub async fn check_if_device_already_logged_in(
        pool: &DbPool,
        device_login_event: Self,
    ) -> Result<Option<Self>, anyhow::Error> {
        let existing_login_event =
            Self::find_device_login_event(&pool, &device_login_event).await?;

        if let None = existing_login_event {
            let mut login_event: DeviceLoginEvent = DeviceLoginEvent::from(device_login_event);
            login_event.save(pool).await?;
            return Ok(Some(login_event));
        }

        Ok(None)
    }
}
