use sqlx::error::Error as SqlxError;
use thiserror::Error;

/// LDAP server responses (especially `LdapResult.text` and `LdapResult.matched`) may contain
/// null bytes and non-printable control characters that corrupt log output. This function
/// filters out all control characters except `\n` and `\t`.
pub(super) fn sanitize_ldap_string(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

#[derive(Debug, Error)]
pub enum LdapError {
    // Stores a sanitized string to avoid null bytes / control chars from LDAP responses
    // corrupting log output.
    #[error("LDAP error: {0}")]
    Ldap(String),
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

impl From<ldap3::LdapError> for LdapError {
    fn from(err: ldap3::LdapError) -> Self {
        Self::Ldap(sanitize_ldap_string(&err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_ldap_string;

    #[test]
    fn sanitize_ldap_string_strips_control_chars() {
        // Null bytes are stripped
        assert_eq!(sanitize_ldap_string("hello\0world"), "helloworld");

        // Other non-printable control chars are stripped
        assert_eq!(sanitize_ldap_string("text\x01\x02\x03"), "text");

        // Realistic LDAP error string containing null bytes is cleaned correctly
        let dirty = "80090308: LdapErr: DSID-0C09044E, comment: AcceptSecurityContext error, data 52e, v2580\0";
        let clean = "80090308: LdapErr: DSID-0C09044E, comment: AcceptSecurityContext error, data 52e, v2580";
        assert_eq!(sanitize_ldap_string(dirty), clean);

        // \n and \t are preserved
        assert_eq!(
            sanitize_ldap_string("line1\nline2\ttabbed"),
            "line1\nline2\ttabbed"
        );

        // Normal ASCII and Unicode pass through unchanged
        assert_eq!(sanitize_ldap_string("hello world"), "hello world");
        assert_eq!(
            sanitize_ldap_string("zażółć gęślą jaźń"),
            "zażółć gęślą jaźń"
        );
    }
}
