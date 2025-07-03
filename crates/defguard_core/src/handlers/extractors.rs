use axum::{
    extract::{FromRef, FromRequestParts, Path},
    http::request::Parts,
};

use crate::{
    appstate::AppState,
    db::{Id, WireguardNetwork},
    error::WebError,
};

impl<S> FromRequestParts<S> for WireguardNetwork<Id>
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let appstate = AppState::from_ref(state);
        let Path(location_id): Path<Id> = Path::from_request_parts(parts, state)
            .await
            .map_err(|_| WebError::ObjectNotFound("Location ID not found in path".to_string()))?;

        WireguardNetwork::find_by_id(&appstate.pool, location_id)
            .await?
            .ok_or_else(|| WebError::ObjectNotFound(format!("Location {location_id} not found")))
    }
}
