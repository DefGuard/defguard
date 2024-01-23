use std::{error::Error, fmt};

#[derive(Debug)]
pub enum LdapError {
    Ldap(String),
    ObjectNotFound(String),
    MissingSettings,
}

impl fmt::Display for LdapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LdapError::Ldap(msg) => write!(f, "LDAP error: {msg}"),
            LdapError::ObjectNotFound(msg) => write!(f, "Object not found: {msg}"),
            LdapError::MissingSettings => {
                write!(f, "LDAP settings are missing.")
            }
        }
    }
}

impl Error for LdapError {}

impl From<ldap3::LdapError> for LdapError {
    fn from(error: ldap3::LdapError) -> Self {
        Self::Ldap(error.to_string())
    }
}
