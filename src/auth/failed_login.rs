// #[derive(Error)]
// enum FailedLoginError(
//
// )

use chrono::{DateTime, Local};
use std::collections::HashMap;

pub struct FailedLoginMap(HashMap<String, FailedLogin>);

pub struct FailedLogin {
    attempt_count: u8,
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
    fn increment(&mut self) {
        // if Local::now().signed_duration_since(self.last_attempt) > Duration::seconds(60) {
        //
        // }
        // self.attempt_count += 1;
        // self.last_attempt = Local::now()
    }

    fn reset(&mut self) {
        self.attempt_count = 1;
        self.first_attempt = Local::now();
        self.last_attempt = Local::now();
    }
}

impl FailedLoginMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn log_failed_attempt(&mut self, username: &str) {
        unimplemented!()
        // info!("Logging failed login attempt for username {}", username);
        // match self.0.get_mut(username) {
        //     None => self.0.insert(username.into(), FailedLogin::default()),
        //     Some(failed_login) => failed_login.increment(),
        // };
    }

    pub fn verify_username(&mut self, username: &str) {
        unimplemented!()
        // if let Some(failed_login) = self.0.get(username) {
        //     if failed_login.attempt_count >= 10 && Local::now().signed_duration_since(failed_login.last_attempt)
        // }
        // Ok(())
    }
}
