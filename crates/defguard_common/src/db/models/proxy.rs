use chrono::NaiveDateTime;
use model_derive::Model;

use crate::db::{Id, NoId};

#[derive(Model)]
pub struct Proxy<I = NoId> {
	pub id: I,
	pub name: String,
	pub address: String,
	pub port: i32,
	pub public_address: String,
	pub connected_at: Option<NaiveDateTime>,
	pub disconnected_at: Option<NaiveDateTime>,
}
