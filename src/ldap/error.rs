use sqlx::error::Error as SqlxError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LdapError {
    #[error("LDAP error: {0}")]
    Ldap(#[from] ldap3::LdapError),
    #[error("Object not found: {0}")]
    ObjectNotFound(String),
    #[error("Missing required LDAP settings: {0}")]
    MissingSettings(String),
    #[error("Found multiple objects, expected one")]
    TooManyObjects,
    #[error("Database error: {0}")]
    Database(#[from] SqlxError),
    #[error("Expected different DN: {0}")]
    InvalidDN(String),
    #[error("Missing attribute: {0}")]
    MissingAttribute(String),
    #[error("LDAP is desynced, awaiting full sync")]
    Desynced,
}
