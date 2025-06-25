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
    #[error("Enterprise features are disabled, not performing LDAP operation: {0}")]
    EnterpriseDisabled(String),
    #[error(
        "User's username \"{0}\" is invalid and cannot be used in Defguard, you can try \
    changing your LDAP username attribute or changing the username in LDAP to a valid one"
    )]
    InvalidUsername(String),
    #[error("LDAP object already exists: {0}")]
    ObjectAlreadyExists(String),
    #[error("User {0} does not belong to the defined synchronization groups in {1}")]
    UserNotInLDAPSyncGroups(String, &'static str),
}
