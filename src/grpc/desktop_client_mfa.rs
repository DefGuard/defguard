use crate::db::{DbPool, Device, User, UserInfo, WireguardNetwork};
use crate::handlers::mail::send_email_mfa_code_email;
use crate::mail::Mail;
use tokio::sync::mpsc::UnboundedSender;
use tonic::Status;
use uuid::Uuid;

use super::proto::{
    ClientMfaFinishRequest, ClientMfaFinishResponse, ClientMfaStartRequest, ClientMfaStartResponse,
    MfaMethod,
};

pub(super) struct ClientMfaServer {
    pool: DbPool,
    mail_tx: UnboundedSender<Mail>,
}

impl ClientMfaServer {
    #[must_use]
    pub fn new(pool: DbPool, mail_tx: UnboundedSender<Mail>) -> Self {
        Self { pool, mail_tx }
    }

    pub async fn start_client_mfa_login(
        &self,
        request: ClientMfaStartRequest,
    ) -> Result<ClientMfaStartResponse, Status> {
        info!("Starting desktop client login: {request:?}");
        // fetch location
        let Ok(Some(location)) =
            WireguardNetwork::find_by_id(&self.pool, request.location_id).await
        else {
            error!("Failed to find location with ID {}", request.location_id);
            return Err(Status::invalid_argument("location not found"));
        };

        // fetch device
        let Ok(Some(device)) = Device::find_by_pubkey(&self.pool, &request.pubkey).await else {
            error!("Failed to find device with pubkey {}", request.pubkey);
            return Err(Status::invalid_argument("device not found"));
        };

        // fetch user
        let Ok(Some(user)) = User::find_by_id(&self.pool, device.user_id).await else {
            error!("Failed to find user with ID {}", device.user_id);
            return Err(Status::invalid_argument("user not found"));
        };
        let user_info = UserInfo::from_user(&self.pool, &user).await.map_err(|_| {
            error!("Failed to fetch user info for {}", user.username);
            Status::internal("unexpected error")
        })?;

        // validate user is allowed to connect to a given location
        let mut transaction = self.pool.begin().await.map_err(|_| {
            error!("Failed to begin transaction");
            Status::internal("unexpected error")
        })?;
        let allowed_groups = location
            .get_allowed_groups(&mut transaction)
            .await
            .map_err(|err| {
                error!("Failed to fetch allowed groups for location {location}: {err:?}");
                Status::internal("unexpected error")
            })?;
        if let Some(groups) = allowed_groups {
            // check if user belongs to one of allowed groups
            if !groups
                .iter()
                .any(|allowed_group| user_info.groups.contains(allowed_group))
            {
                error!(
                    "User {} not allowed to connect to location {location}",
                    user.username
                );
                return Err(Status::unauthenticated("unauthorized"));
            }
        }

        // check if selected method is enabled
        match MfaMethod::try_from(request.method) {
            Ok(MfaMethod::Totp) => {
                if !user.totp_enabled {
                    error!("TOTP not enabled for user {}", user.username);
                    return Err(Status::invalid_argument(
                        "selected MFA method not available",
                    ));
                }
            }
            Ok(MfaMethod::Email) => {
                if !user.email_mfa_enabled {
                    error!("Email MFA not enabled for user {}", user.username);
                    return Err(Status::invalid_argument(
                        "selected MFA method not available",
                    ));
                }
                // send email code
                send_email_mfa_code_email(&user, &self.mail_tx, None).map_err(|err| {
                    error!(
                        "Failed to send email MFA code for user {}: {err:?}",
                        user.username
                    );
                    Status::internal("unexpected error")
                })?;
            }
            Err(err) => {
                error!("Invalid MFA method selected: {err}");
                return Err(Status::invalid_argument("invalid MFA method selected"));
            }
        }

        // generate auth token
        let token = Uuid::new_v4().into();

        // store login session
        todo!();

        Ok(ClientMfaStartResponse { token })
    }

    pub async fn finish_client_mfa_login(
        &self,
        request: ClientMfaFinishRequest,
    ) -> Result<ClientMfaFinishResponse, Status> {
        todo!()
    }
}
