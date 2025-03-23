use std::future::Future;

use sqlx::PgPool;
use sync::{get_ldap_sync_status, is_ldap_desynced, set_ldap_sync_status, SyncStatus};

use crate::{
    db::Settings,
    enterprise::{is_enterprise_enabled, limits::update_counts},
    ldap::{error::LdapError, LDAPConnection},
};

pub mod model;
pub mod sync;

pub async fn do_ldap_sync(pool: &PgPool) -> Result<(), LdapError> {
    debug!("Starting LDAP sync, if enabled");
    let settings = Settings::get_current_settings();
    if !settings.ldap_enabled {
        debug!("LDAP is disabled, not performing LDAP sync");
        return Ok(());
    }
    if !settings.ldap_sync_enabled {
        debug!("LDAP sync is disabled, not performing LDAP sync");
        return Ok(());
    }
    if !is_enterprise_enabled() {
        debug!("Enterprise features are disabled, not performing LDAP sync");
        return Err(LdapError::EnterpriseDisabled("LDAP sync".to_string()));
    }

    if is_ldap_desynced() {
        info!("LDAP is considered to be desynced, doing a full sync");
    } else {
        info!("Ldap is not considered to be desynced, doing an incremental sync");
    }

    let mut ldap_connection = match LDAPConnection::create().await {
        Ok(connection) => connection,
        Err(err) => {
            set_ldap_sync_status(SyncStatus::Desynced, pool).await?;
            return Err(err);
        }
    };

    if let Err(err) = ldap_connection.sync(pool, is_ldap_desynced()).await {
        set_ldap_sync_status(SyncStatus::Desynced, pool).await?;
        return Err(err);
    } else {
        set_ldap_sync_status(SyncStatus::Synced, pool).await?;
    };

    let _ = update_counts(pool).await;

    info!("LDAP sync completed");

    Ok(())
}

/// Convenience function to run a function that performs an LDAP operation and handle the result
/// appropriately, setting the LDAP sync status to Desynced if an error is encountered.
pub async fn with_ldap_status<T, F>(pool: &PgPool, f: F) -> Result<T, LdapError>
where
    F: Future<Output = Result<T, LdapError>>,
{
    let settings = Settings::get_current_settings();
    if !settings.ldap_enabled {
        debug!("LDAP is disabled, not performing LDAP operation");
        return Err(LdapError::MissingSettings("LDAP is disabled".into()));
    }

    if settings.ldap_sync_enabled && get_ldap_sync_status() == SyncStatus::Desynced {
        warn!("LDAP is considered to be desynced, not performing LDAP operation");
        return Err(LdapError::Desynced);
    }

    match f.await {
        Ok(result) => Ok(result),
        Err(e) => {
            warn!(
                "Encountered an error while performing LDAP operation: {:?}",
                e
            );

            if let Err(status_err) = set_ldap_sync_status(SyncStatus::Desynced, pool).await {
                warn!("Failed to update LDAP sync status: {:?}", status_err);
            }

            Err(e)
        }
    }
}
