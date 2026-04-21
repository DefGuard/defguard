use defguard_common::db::models::settings::SettingsSaveError;
use thiserror::Error;

/// Strips null bytes and non-printable control characters from LDAP error strings.
/// LDAP server responses may embed raw binary data or null terminators in diagnostic
/// fields (e.g. `LdapResult.text`, `LdapResult.matched`) that corrupt log output.
pub(super) fn sanitize_ldap_string(s: &str) -> String {
    s.chars()
        .filter(|&c| c == '\n' || c == '\t' || !c.is_control())
        .collect()
}

#[derive(Debug, Error)]
pub enum LdapError {
    /// Sanitized string representation of an ldap3 error. Stored as String (rather than
    /// the original ldap3::LdapError) so null bytes and control chars are stripped once
    /// at conversion time and can never surface through Display or Debug.
    #[error("LDAP error: {0}")]
    Ldap(String),
    #[error("Object not found: {0}")]
    ObjectNotFound(String),
    #[error("Missing required LDAP settings: {0}")]
    MissingSettings(String),
    #[error("Found multiple objects, expected one")]
    TooManyObjects,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    SettingsSave(#[from] SettingsSaveError),
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
    #[error("License user limit reached: {0}/{1}")]
    LicenseUserLimitReached(u32, u32),
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
        // Null bytes, bells, and other control chars should be removed.
        assert_eq!(sanitize_ldap_string("hello\0world"), "helloworld");
        assert_eq!(sanitize_ldap_string("text\x01\x02\x03"), "text");
        assert_eq!(
            sanitize_ldap_string("rc=49, dn: \"\0\", text: \"invalid\0creds\""),
            "rc=49, dn: \"\", text: \"invalidcreds\""
        );

        // Tabs and newlines should be kept.
        assert_eq!(
            sanitize_ldap_string("line1\nline2\ttabbed"),
            "line1\nline2\ttabbed"
        );

        // Printable ASCII and Unicode should pass through unchanged.
        assert_eq!(
            sanitize_ldap_string("normal error text"),
            "normal error text"
        );
        assert_eq!(sanitize_ldap_string("ünïcödé"), "ünïcödé");
    }
}
