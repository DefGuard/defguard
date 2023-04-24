use crate::{db::models::error::ModelError, ldap::error::OriLDAPError};
use sqlx::error::Error as SqlxError;
use std::{error::Error, fmt};

/// Represents kinds of error that occurred
#[derive(Debug)]
pub enum OriWebError {
    Grpc(String),
    Ldap(String),
    WebauthnRegistration(String),
    IncorrectUsername(String),
    ObjectNotFound(String),
    Serialization(String),
    Authorization(String),
    Forbidden(String),
    DbError(String),
    ModelError(String),
    Http(rocket::http::Status),
}

impl fmt::Display for OriWebError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OriWebError::Grpc(msg) => write!(f, "GRPC error: {}", msg),
            OriWebError::Ldap(msg) => write!(f, "LDAP error: {}", msg),
            OriWebError::WebauthnRegistration(msg) => {
                write!(f, "Webauthn registration error: {}", msg)
            }
            OriWebError::IncorrectUsername(username) => {
                write!(f, "Incorrect username: {}", username)
            }
            OriWebError::ObjectNotFound(msg) => write!(f, "Object not found: {}", msg),
            OriWebError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            OriWebError::Authorization(msg) => write!(f, "Authorization error: {}", msg),
            OriWebError::Forbidden(msg) => write!(f, "Forbidden error: {}", msg),
            OriWebError::DbError(msg) => write!(f, "Database error: {}", msg),
            OriWebError::ModelError(msg) => write!(f, "Model error: {}", msg),
            OriWebError::Http(status) => write!(f, "HTTP error: {}", status),
        }
    }
}

impl Error for OriWebError {}

impl From<tonic::Status> for OriWebError {
    fn from(status: tonic::Status) -> Self {
        Self::Grpc(status.message().into())
    }
}

impl From<rocket::http::Status> for OriWebError {
    fn from(status: rocket::http::Status) -> Self {
        Self::Http(status)
    }
}

impl From<OriLDAPError> for OriWebError {
    fn from(error: OriLDAPError) -> Self {
        match error {
            OriLDAPError::ObjectNotFound(msg) => Self::ObjectNotFound(msg),
            OriLDAPError::Ldap(msg) => Self::Ldap(msg),
        }
    }
}

impl From<SqlxError> for OriWebError {
    fn from(error: SqlxError) -> Self {
        Self::DbError(error.to_string())
    }
}

impl From<ModelError> for OriWebError {
    fn from(error: ModelError) -> Self {
        Self::ModelError(error.to_string())
    }
}
