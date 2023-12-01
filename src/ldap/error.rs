use ldap3::LdapError;
use std::{error::Error, fmt};

#[derive(Debug)]
pub enum OriLDAPError {
    Ldap(String),
    ObjectNotFound(String),
    MissingSettings,
}

impl fmt::Display for OriLDAPError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OriLDAPError::Ldap(msg) => write!(f, "LDAP error: {msg}"),
            OriLDAPError::ObjectNotFound(msg) => write!(f, "Object not found: {msg}"),
            OriLDAPError::MissingSettings => {
                write!(f, "LDAP settings are missing.")
            }
        }
    }
}

impl Error for OriLDAPError {}

impl From<LdapError> for OriLDAPError {
    fn from(error: LdapError) -> Self {
        Self::Ldap(error.to_string())
    }
}
