use std::{error::Error, fmt};

#[derive(Debug)]
pub enum LdapError {
    Ldap(String),
    ObjectNotFound(String),
    MissingSettings,
    TooManyObjects,
    // TODO: include the error
    Database,
}

impl fmt::Display for LdapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ldap(msg) => write!(f, "LDAP error: {msg}"),
            Self::ObjectNotFound(msg) => write!(f, "Object not found: {msg}"),
            Self::MissingSettings => {
                write!(f, "LDAP settings are missing")
            }
            Self::TooManyObjects => write!(f, "Found multiple objects"),
            Self::Database => write!(f, "Database error"),
        }
    }
}

impl Error for LdapError {}

impl From<ldap3::LdapError> for LdapError {
    fn from(error: ldap3::LdapError) -> Self {
        Self::Ldap(error.to_string())
    }
}
