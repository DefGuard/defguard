use error::UserSnatBindingError;
use sqlx::query_as;
use std::net::IpAddr;

use crate::db::{Id, WireguardNetwork};

use super::db::models::snat::UserSnatBinding;

pub mod error;
pub mod handlers;

impl WireguardNetwork<Id> {
    async fn get_all_snat_bindings<'e, E: sqlx::PgExecutor<'e>>(
        &self,
        executor: E,
    ) -> Result<Vec<UserSnatBinding<Id>>, UserSnatBindingError> {
        debug!("Fetching all SNAT bindings for location {self}");

        let bindings = query_as!(
        UserSnatBinding::<Id>,
        "SELECT id, user_id, location_id, \"public_ip\" \"public_ip: IpAddr\" FROM user_snat_binding WHERE location_id = $1",
        self.id
	    )
	    .fetch_all(executor)
	    .await?;

        Ok(bindings)
    }
}
