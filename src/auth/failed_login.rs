use chrono::{DateTime, Duration, Local};
use std::collections::HashMap;
use thiserror::Error;

// Time window in seconds
const FAILED_LOGIN_WINDOW: u32 = 60;
// Failed login count threshold
const FAILED_LOGIN_COUNT: u32 = 5;
// How long (in seconds) to lock users out after crossing the threshold
const FAILED_LOGIN_TIMEOUT: u32 = 5 * 60;

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
        self.last_attempt = Local::now()
    }

    fn reset(&mut self) {
        self.attempt_count = 1;
        self.first_attempt = Local::now();
        self.last_attempt = Local::now();
    }
}

impl Default for FailedLoginMap {
    fn default() -> Self {
        Self::new()
    }
}

impl FailedLoginMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    // Add failed login attempt to tracker
    pub fn log_failed_attempt(&mut self, username: &str) {
        info!("Logging failed login attempt for username {}", username);
        match self.0.get_mut(username) {
            None => {
                self.0.insert(username.into(), FailedLogin::default());
            }
            Some(failed_login) => {
                // reset counter if enough time has elapsed since first attempt
                // if the attempt count threshold has been reached
                // check if the timeout since last attempt has also ended
                if failed_login.time_since_first_attempt()
                    > Duration::seconds(FAILED_LOGIN_WINDOW as i64)
                    && (failed_login.attempt_count < FAILED_LOGIN_COUNT
                        || failed_login.time_since_last_attempt()
                            > Duration::seconds(FAILED_LOGIN_TIMEOUT as i64))
                {
                    failed_login.reset()
                } else {
                    failed_login.increment()
                }
            }
        };
    }

    // Check if user can proceed with login process or should be locked out
    pub fn verify_username(&mut self, username: &str) -> Result<(), FailedLoginError> {
        if let Some(failed_login) = self.0.get_mut(username) {
            if failed_login.attempt_count >= 10
                && failed_login.time_since_last_attempt()
                    <= Duration::seconds(FAILED_LOGIN_TIMEOUT as i64)
            {
                // log a failed attempt to prolong timeout
                failed_login.increment();
                return Err(FailedLoginError);
            }
        }
        Ok(())
    }
}
