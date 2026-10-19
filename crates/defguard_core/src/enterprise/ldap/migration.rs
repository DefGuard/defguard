use sqlx::{PgPool, query, query_as};
use tracing::info;

use super::{
    dn::{find_unescaped_separator, unescape_value},
    error::LdapError,
};

#[derive(sqlx::FromRow)]
struct Candidate {
    id: i64,
    username: String,
    ldap_rdn: String,
    ldap_user_path: Option<String>,
}

#[derive(serde::Serialize)]
pub struct LdapRepairCounts {
    pub found: u64,
    pub migrated: u64,
}

/// Rebuilds the legacy value and path split at the first unescaped comma.
fn repaired_dn(
    ldap_rdn: &str,
    ldap_user_path: Option<&str>,
) -> Result<(String, String), LdapError> {
    let path =
        ldap_user_path.ok_or_else(|| LdapError::InvalidDN("missing LDAP user path".into()))?;
    // reconstruct the full DN from the stored legacy value and path before decoding
    let full_dn = format!("{ldap_rdn},{path}");
    let separator = find_unescaped_separator(&full_dn, b',')
        .ok_or_else(|| LdapError::InvalidDN(full_dn.clone()))?;
    let value = unescape_value(&full_dn[..separator])
        .ok_or_else(|| LdapError::InvalidDN(full_dn.clone()))?;
    Ok((value, full_dn[separator + 1..].to_owned()))
}

/// Updates both DN fields atomically using the stored LDAP DN components.
pub async fn repair(pool: &PgPool) -> Result<LdapRepairCounts, LdapError> {
    info!("Starting LDAP user DN repair");
    let candidates: Vec<Candidate> = query_as(
        "SELECT id, username, ldap_rdn, ldap_user_path FROM \"user\" WHERE from_ldap AND POSITION(chr(92) IN ldap_rdn) > 0",
    )
    .fetch_all(pool)
    .await?;
    info!("Found {} LDAP users requiring repair", candidates.len());
    if candidates.is_empty() {
        return Ok(LdapRepairCounts {
            found: 0,
            migrated: 0,
        });
    }

    let mut transaction = pool.begin().await?;
    let mut migrated = 0;
    for candidate in &candidates {
        debug!("Repairing LDAP user {}", candidate.username);
        let (ldap_rdn, ldap_user_path) =
            repaired_dn(&candidate.ldap_rdn, candidate.ldap_user_path.as_deref())?;
        let result = query("UPDATE \"user\" SET ldap_rdn = $1, ldap_user_path = $2 WHERE id = $3 AND (ldap_rdn IS DISTINCT FROM $1 OR ldap_user_path IS DISTINCT FROM $2)")
            .bind(&ldap_rdn)
            .bind(&ldap_user_path)
            .bind(candidate.id)
            .execute(&mut *transaction)
            .await?;
        migrated += result.rows_affected();
        info!("Repaired LDAP user {}", candidate.username);
    }
    transaction.commit().await?;
    info!(
        "User migration completed: found {}. Successfully migrated: {}.",
        candidates.len(),
        migrated,
    );
    Ok(LdapRepairCounts {
        found: candidates.len() as u64,
        migrated,
    })
}

#[cfg(test)]
mod tests {
    use super::repaired_dn;

    #[test]
    fn repairs_legacy_split_at_escaped_comma() {
        assert_eq!(
            repaired_dn(r"Doe\", Some(" John,ou=users,dc=example,dc=org")).unwrap(),
            ("Doe, John".into(), "ou=users,dc=example,dc=org".into())
        );
    }

    #[test]
    fn decodes_hex_commas_and_preserves_escaped_parent_path() {
        let path = r"ou=People\,West,dc=example";
        for rdn in [r"Doe\2C John", r"Doe\2c John"] {
            assert_eq!(
                repaired_dn(rdn, Some(path)).unwrap(),
                ("Doe, John".into(), path.into())
            );
        }
    }

    #[test]
    fn preserves_plain_value_without_attribute_prefix() {
        assert_eq!(
            repaired_dn("Jane", Some("ou=users,dc=example")).unwrap(),
            ("Jane".into(), "ou=users,dc=example".into())
        );
    }

    #[test]
    fn rejects_malformed_value_and_missing_path() {
        assert!(repaired_dn(r"Doe\", Some(" John")).is_err());
        assert!(repaired_dn(r"Doe\FF", Some("ou=users,dc=example")).is_err());
        assert!(repaired_dn(r"Doe\2C John", None).is_err());
    }
}
