use std::net::IpAddr;

use crate::db::{Id, NoId};
use error::UserSnatBindingError;
use model_derive::Model;
use serde::Serialize;
use sqlx::{query_as, PgExecutor};
use utoipa::ToSchema;

pub mod error;
pub mod handlers;

#[derive(Debug, Model, Serialize, ToSchema)]
#[table(user_snat_binding)]
pub struct UserSnatBinding<I = NoId> {
    pub id: I,
    pub user_id: Id,
    pub location_id: Id,
    #[model(ip)]
    #[schema(value_type = String)]
    pub public_ip: IpAddr,
}

impl UserSnatBinding {
    pub fn new(user_id: Id, location_id: Id, public_ip: IpAddr) -> Self {
        Self {
            id: NoId,
            user_id,
            location_id,
            public_ip,
        }
    }
}

impl UserSnatBinding<Id> {
    pub async fn find_binding<'e, E>(
        executor: E,
        location_id: Id,
        user_id: Id,
    ) -> Result<Self, UserSnatBindingError>
    where
        E: PgExecutor<'e>,
    {
        let binding = query_as!(Self,
        "SELECT id, user_id, location_id, \"public_ip\" \"public_ip: IpAddr\" FROM user_snat_binding WHERE location_id = $1 AND user_id = $2",
        location_id, user_id
    	).fetch_one(executor).await?;

        Ok(binding)
    }

    pub fn update_ip(&mut self, new_public_ip: IpAddr) {
        self.public_ip = new_public_ip;
    }
}
