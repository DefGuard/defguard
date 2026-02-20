use sqlx::Postgres;

pub mod models;

pub trait PgAcquire<'a>: sqlx::Acquire<'a, Database = Postgres> {}
