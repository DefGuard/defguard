use std::{collections::HashMap, sync::Mutex};

use chrono::{DateTime, Duration, Local};
use thiserror::Error;

// Time window in seconds
const FAILED_LOGIN_WINDOW: i64 = 60;
// Failed login count threshold
const FAILED_LOGIN_COUNT: u32 = 5;
// How long (in seconds) to lock users out after crossing the threshold
const FAILED_LOGIN_TIMEOUT: i64 = 5 * 60;

#[derive(Error, Debug)]
#[error("Too many login attempts")]
pub struct FailedLoginError;

pub struct FailedLoginMap(HashMap<String, FailedLogin>);

pub struct FailedLogin {
    attempt_count: u32,
    first_attempt: DateTime<Local>,
    last_attempt: DateTime<Local>,
}

impl Default for FailedLogin {
    fn default() -> Self {
        FailedLogin {
            attempt_count: 1,
            first_attempt: Local::now(),
            last_attempt: Local::now(),
        }
    }
}

impl FailedLogin {
    // How much time has elapsed since first failed login attempt
    fn time_since_first_attempt(&self) -> Duration {
        Local::now().signed_duration_since(self.first_attempt)
    }

    // How much time has elapsed since last failed login attempt
    fn time_since_last_attempt(&self) -> Duration {
        Local::now().signed_duration_since(self.last_attempt)
    }

    fn increment(&mut self) {
        self.attempt_count += 1;
        self.last_attempt = Local::now();
    }

    fn reset(&mut self) {
        self.attempt_count = 1;
        self.first_attempt = Local::now();
        self.last_attempt = Local::now();
    }

    // Check if user login attempt should be stopped
    fn should_prevent_login(&self) -> bool {
        self.attempt_count >= FAILED_LOGIN_COUNT
            && self.time_since_last_attempt() <= Duration::seconds(FAILED_LOGIN_TIMEOUT)
    }

    // Check if attempt counter can be reset.
    // Counter can be reset after enough time has passed since the initial attempt.
    // If user was blocked we also check if enough time (timeout) has passed since last attempt.
    fn should_reset_counter(&self) -> bool {
        self.time_since_first_attempt() > Duration::seconds(FAILED_LOGIN_WINDOW)
            && self.attempt_count < FAILED_LOGIN_COUNT
            || self.time_since_last_attempt() > Duration::seconds(FAILED_LOGIN_TIMEOUT)
    }
}

impl Default for FailedLoginMap {
    fn default() -> Self {
        Self::new()
    }
}

impl FailedLoginMap {
    #[must_use]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    // Add failed login attempt to tracker
    pub fn log_failed_attempt(&mut self, username: &str) {
        info!("Logging failed login attempt for username {username}");
        match self.0.get_mut(username) {
            None => {
                self.0.insert(username.into(), FailedLogin::default());
            }
            Some(failed_login) => {
                if failed_login.should_reset_counter() {
                    failed_login.reset();
                } else {
                    failed_login.increment();
                }
            }
        };
    }

    // Check if user can proceed with login process or should be locked out
    pub fn verify_username(&mut self, username: &str) -> Result<(), FailedLoginError> {
        debug!("Checking if user {username} can proceed with login");
        if let Some(failed_login) = self.0.get_mut(username) {
            if failed_login.should_prevent_login() {
                debug!("Preventing user {username} from logging in");
                // log a failed attempt to prolong timeout
                failed_login.increment();
                return Err(FailedLoginError);
            }
        }
        Ok(())
    }
}

// Check if auth request with a given username can proceed
pub fn check_username(
    failed_logins: &Mutex<FailedLoginMap>,
    username: &str,
) -> Result<(), FailedLoginError> {
    let mut failed_logins = failed_logins
        .lock()
        .expect("Failed to get a lock on failed login map.");
    failed_logins.verify_username(username)
}

// Helper to log failed login attempt
pub fn log_failed_login_attempt(failed_logins: &Mutex<FailedLoginMap>, username: &str) {
    let mut failed_logins = failed_logins
        .lock()
        .expect("Failed to get a lock on failed login map.");
    failed_logins.log_failed_attempt(username);
}
