use tokio::sync::broadcast::Sender;

use defguard_enterprise_directory_sync::{DirectorySyncContext, DirectorySyncError};

use crate::{grpc::GatewayEvent, user_management};

pub fn build_directory_sync_context(wg_tx: Sender<GatewayEvent>) -> DirectorySyncContext {
    let disable_tx = wg_tx.clone();
    let delete_tx = wg_tx.clone();
    let sync_tx = wg_tx.clone();
    DirectorySyncContext {
        disable_user: Box::new(move |user, conn| {
            let disable_tx = disable_tx.clone();
            Box::pin(async move {
                user_management::disable_user(user, conn, &disable_tx)
                    .await
                    .map_err(|err| DirectorySyncError::UserUpdateError(err.to_string()))
            })
        }),
        delete_user_and_cleanup_devices: Box::new(move |user, conn| {
            let delete_tx = delete_tx.clone();
            Box::pin(async move {
                user_management::delete_user_and_cleanup_devices(user, conn, &delete_tx)
                    .await
                    .map_err(|err| DirectorySyncError::UserUpdateError(err.to_string()))
            })
        }),
        sync_allowed_user_devices: Box::new(move |user, conn| {
            let sync_tx = sync_tx.clone();
            Box::pin(async move {
                user_management::sync_allowed_user_devices(user, conn, &sync_tx)
                    .await
                    .map_err(|err| DirectorySyncError::NetworkUpdateError(err.to_string()))
            })
        }),
    }
}
