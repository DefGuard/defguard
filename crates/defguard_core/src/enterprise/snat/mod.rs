use std::net::IpAddr;

use crate::db::{Id, NoId};
use model_derive::Model;
use serde::Serialize;

pub mod error;
pub mod handlers;

#[derive(Debug, Model, Serialize)]
#[table(user_snat_binding)]
pub struct UserSnatBinding<I = NoId> {
    pub id: I,
    pub user_id: Id,
    pub location_id: Id,
    #[model(ip)]
    pub public_ip: IpAddr,
}
