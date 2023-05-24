use std::{collections::HashMap, time::Duration as StdDuration};

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::error::OriWebError;

pub struct OpenIdSessionMap {
    map: HashMap<String, OpenIdSession>,
}

// sets how much time, session can live, in seconds
const OPENID_SESSION_LIFETIME: u64 = 60 * 10;

#[derive(Debug, Clone)]
pub struct OpenIdSession {
    pub redirect_url: String,
    pub lifetime: StdDuration,
    created: DateTime<Utc>,
}

impl OpenIdSession {
    #[must_use]
    pub fn new(redirect_url: String) -> OpenIdSession {
        OpenIdSession {
            redirect_url,
            created: Utc::now(),
            lifetime: StdDuration::new(OPENID_SESSION_LIFETIME, 0),
        }
    }

    pub fn expired(&self) -> bool {
        let now = Utc::now();
        let duration = Duration::from_std(self.lifetime)
            .expect("Failed to convert session lifetime duration.");
        if let Some(expiration) = self.created.checked_add_signed(duration) {
            return expiration <= now;
        }
        false
    }
}

impl OpenIdSessionMap {
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    // adds item to map and returns it's key
    pub fn add(&mut self, session: OpenIdSession) -> Result<String, OriWebError> {
        let session_id = Uuid::new_v4().to_string();
        match self.map.insert(session_id.clone(), session) {
            Some(_) => Err(OriWebError::WebError(
                "Session key collision during openId flow.".into(),
            )),
            None => Ok(session_id),
        }
    }

    // Removes session and returns it if it did not expired
    pub fn remove(&mut self, key: &String) -> Option<OpenIdSession> {
        if let Some(session) = self.map.remove(key) {
            if !session.expired() {
                return Some(session);
            }
        }
        None
    }
}

impl Default for OpenIdSessionMap {
    fn default() -> Self {
        Self::new()
    }
}
