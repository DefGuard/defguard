use sqlx::{PgPool, query, query_as};
use tracing::info;

use super::{LDAPConnection, error::LdapError};

#[derive(sqlx::FromRow)]
struct Candidate {
    id: i64,
    username: String,
}

#[derive(serde::Serialize)]
pub struct LdapRepairCounts {
    pub found: u64,
    pub migrated: u64,
}

/// Updates both DN fields atomically using the current LDAP entries.
pub async fn repair(pool: &PgPool) -> Result<LdapRepairCounts, LdapError> {
    info!("Starting LDAP user DN repair");
    let candidates: Vec<Candidate> = query_as(
        "SELECT id, username FROM \"user\" WHERE from_ldap AND POSITION(chr(92) IN ldap_rdn) > 0",
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

    let mut connection = LDAPConnection::create().await?;
    let mut transaction = pool.begin().await?;
    let mut migrated = 0;
    for candidate in &candidates {
        debug!("Repairing LDAP user {}", candidate.username);
        let ldap_user = connection.get_user_by_username(&candidate.username).await?;
        let result = query("UPDATE \"user\" SET ldap_rdn = $1, ldap_user_path = $2 WHERE id = $3 AND (ldap_rdn IS DISTINCT FROM $1 OR ldap_user_path IS DISTINCT FROM $2)")
            .bind(&ldap_user.ldap_rdn)
            .bind(&ldap_user.ldap_user_path)
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
