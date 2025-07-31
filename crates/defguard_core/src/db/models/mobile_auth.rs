use crate::db::{Id, NoId};
use chrono::NaiveDateTime;
use model_derive::Model;

#[derive(Model, Clone)]
#[table(mobile_auth)]
pub struct MobileAuth<I = NoId> {
    pub id: I,
    pub pub_key: String,
    pub device_id: Id,
}

impl MobileAuth {
    #[must_use]
    pub fn new(device_id: Id, pub_key: String) -> Self {
        Self {
            id: NoId,
            device_id,
            pub_key,
        }
    }
}

#[derive(Model, Clone)]
#[table(mobile_challenge)]
pub struct MobileChallenge<I = NoId> {
    pub id: I,
    pub auth_id: Option<Id>,
    pub challenge: String,
    pub created_at: NaiveDateTime,
}
