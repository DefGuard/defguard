use crate::random::gen_alphanumeric;
use chrono::{Duration, NaiveDateTime, Utc};

pub const ENROLLMENT_TOKEN_TIMEOUT: i64 = 60 * 60 * 24; // token is valid for 24h
pub const ENROLLMENT_SESSION_TIMEOUT: i64 = 60 * 10; // session is valid for 10m

// Representation of a user enrollment session
#[derive(Clone)]
pub struct Enrollment {
    pub id: String,
    pub user_id: i64,
    pub admin_id: i64,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub used_at: Option<NaiveDateTime>,
}

impl Enrollment {
    pub fn new(user_id: i64, admin_id: i64) -> Self {
        let now = Utc::now();
        Self {
            id: gen_alphanumeric(32),
            user_id,
            admin_id,
            created_at: now.naive_utc(),
            expires_at: (now + Duration::seconds(ENROLLMENT_TOKEN_TIMEOUT)).naive_utc(),
            used_at: None,
        }
    }

    // check if token has already expired
    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now().naive_utc()
    }

    // check if token has already been used
    pub fn is_used(&self) -> bool {
        self.used_at.is_some()
    }

    // check if enrollment session is still valid
    // after using the token user has 10 minutes to complete enrollment
    pub fn is_session_valid(&self) -> bool {
        if let Some(used_at) = self.used_at {
            let now = Utc::now();
            return now.naive_utc() < (used_at + Duration::seconds(ENROLLMENT_SESSION_TIMEOUT));
        }
        false
    }
}
