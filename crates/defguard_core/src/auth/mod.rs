pub mod failed_login;

use axum::{
    extract::{FromRef, FromRequestParts, OptionalFromRequestParts},
    http::request::Parts,
};
use axum_client_ip::InsecureClientIp;
use axum_extra::{
    TypedHeader,
    extract::cookie::CookieJar,
    headers::{Authorization, authorization::Bearer},
};
use defguard_common::db::Id;

use crate::{
    appstate::AppState,
    db::{
        Group, OAuth2Token, Session, SessionState, User,
        models::{group::Permission, oauth2client::OAuth2Client},
    },
    enterprise::{db::models::api_tokens::ApiToken, is_enterprise_enabled},
    error::WebError,
    handlers::SESSION_COOKIE_NAME,
};

pub const TOTP_CODE_VALIDITY_PERIOD: u64 = 30;
pub const EMAIL_CODE_DIGITS: u32 = 6;
pub const TOTP_CODE_DIGITS: u32 = 6;

impl<S> FromRequestParts<S> for Session
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let appstate = AppState::from_ref(state);

        // first try to authenticate by API token if one is found in header
        if is_enterprise_enabled() {
            let maybe_auth_header: Option<TypedHeader<Authorization<Bearer>>> =
                <TypedHeader<_> as OptionalFromRequestParts<S>>::from_request_parts(parts, state)
                    .await
                    .map_err(|err| {
                        error!("Failed to extract optional auth header: {err}");
                        WebError::Authorization("Invalid auth header".into())
                    })?;
            if let Some(header) = maybe_auth_header {
                let token_string = header.token();
                debug!("Trying to authorize request using API token: {token_string}");
                return match ApiToken::try_find_by_auth_token(&appstate.pool, token_string).await {
                    Ok(Some(api_token)) => {
                        // create a dummy session and don't store it in the DB
                        // since each request needs to be authorized anyway
                        let ip_address = InsecureClientIp::from_request_parts(parts, state)
                            .await
                            .map_err(|err| {
                            error!("Failed to get client IP: {err:?}");
                            WebError::ClientIpError
                        })?;
                        Ok(Session::new(
                            api_token.user_id,
                            SessionState::ApiTokenVerified,
                            ip_address.0.to_string(),
                            None,
                        ))
                    }
                    Ok(None) => Err(WebError::Authorization("Invalid API token".into())),
                    Err(err) => Err(err.into()),
                };
            }
        }

        let Ok(cookies) = CookieJar::from_request_parts(parts, state).await;
        if let Some(session_cookie) = cookies.get(SESSION_COOKIE_NAME) {
            return {
                match Session::find_by_id(&appstate.pool, session_cookie.value()).await {
                    Ok(Some(session)) => {
                        if session.expired() {
                            let _result = session.delete(&appstate.pool).await;
                            Err(WebError::Authorization("Session expired".into()))
                        } else {
                            Ok(session)
                        }
                    }
                    Ok(None) => Err(WebError::Authorization("Session not found".into())),
                    Err(err) => Err(err.into()),
                }
            };
        }

        Err(WebError::Authorization("Session is required".into()))
    }
}

// Extension of base user session that contains user data fetched from database.
// This represents a session for a user who completed the login process (including MFA, if enabled).
#[derive(Clone)]
pub struct SessionInfo {
    pub session: Session,
    pub user: User<Id>,
    pub is_admin: bool,
    groups: Vec<Group<Id>>,
}

impl SessionInfo {
    #[must_use]
    pub fn new(session: Session, user: User<Id>, is_admin: bool) -> Self {
        Self {
            session,
            user,
            is_admin,
            groups: Vec::new(),
        }
    }

    fn contains_any_group(&self, group_names: &[&str]) -> bool {
        self.groups
            .iter()
            .any(|group| group_names.contains(&group.name.as_str()))
    }
}

impl<S> FromRequestParts<S> for SessionInfo
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state).await?;
        let appstate = AppState::from_ref(state);
        let user = User::find_by_id(&appstate.pool, session.user_id).await?;

        if let Some(user) = user {
            if user.mfa_enabled
                && (session.state != SessionState::MultiFactorVerified
                    && session.state != SessionState::ApiTokenVerified)
            {
                return Err(WebError::Authorization("MFA not verified".into()));
            }
            let Ok(groups) = user.member_of(&appstate.pool).await else {
                return Err(WebError::DbError("cannot fetch groups".into()));
            };
            let is_admin = user.is_admin(&appstate.pool).await?;

            // non-admin users are not allowed to use token auth
            if !is_admin && session.state == SessionState::ApiTokenVerified {
                return Err(WebError::Forbidden(
                    "Token authentication is not allowed for normal users".into(),
                ));
            }

            // Store session info into request extensions so future extractors can use it
            let session_info = SessionInfo {
                session,
                user,
                is_admin,
                groups,
            };
            parts.extensions.insert(session_info.clone());
            Ok(session_info)
        } else {
            Err(WebError::Authorization("User not found".into()))
        }
    }
}

#[macro_export]
macro_rules! role {
    ($name:ident, $($permission:path)*) => {
        pub struct $name;

        impl<S> FromRequestParts<S> for $name
        where
            S: Send + Sync,
            AppState: FromRef<S>,
        {
            type Rejection = WebError;

            async fn from_request_parts(
                parts: &mut Parts,
                state: &S,
            ) -> Result<Self, Self::Rejection> {
                let session_info = SessionInfo::from_request_parts(parts, state).await?;
                if !session_info.user.is_active {
                    return Err(WebError::Forbidden("user is disabled".into()));
                }
                let appstate = AppState::from_ref(state);
                $(
                let groups_with_permission = Group::find_by_permission(
                    &appstate.pool,
                    $permission,
                ).await?;
                let group_names = groups_with_permission.iter().map(|group| group.name.as_str()).collect::<Vec<_>>();
                if session_info.contains_any_group(&group_names) {
                    return Ok(Self {});
                }
                )*
                Err(WebError::Forbidden("access denied".into()))
            }
        }
    };
}

role!(AdminRole, Permission::IsAdmin);

#[derive(Debug)]
pub(crate) struct UserClaims {
    pub email: Option<String>,
    pub family_name: Option<String>,
    pub given_name: Option<String>,
    pub name: Option<String>,
    pub phone_number: Option<String>,
    pub preferred_username: Option<String>,
    pub sub: String,
}

fn get_available_scopes<'a>(
    all_scopes: &'a [String],
    requested_scopes: &'a [String],
) -> Vec<&'a str> {
    let mut scopes = Vec::new();
    for scope in requested_scopes {
        if all_scopes.contains(scope) {
            scopes.push(scope.as_str());
        }
    }
    scopes
}

impl UserClaims {
    pub fn from_user(
        user: &User<Id>,
        oauth_client: &OAuth2Client<Id>,
        oauth_token: &OAuth2Token,
    ) -> Self {
        let token_scopes = oauth_token
            .scope
            .split_whitespace()
            .map(String::from)
            .collect::<Vec<String>>();
        let scopes = get_available_scopes(&oauth_client.scope, &token_scopes);
        Self {
            email: if scopes.contains(&"email") {
                Some(user.email.clone())
            } else {
                None
            },
            family_name: if scopes.contains(&"profile") {
                Some(user.last_name.clone())
            } else {
                None
            },
            given_name: if scopes.contains(&"profile") {
                Some(user.first_name.clone())
            } else {
                None
            },
            name: if scopes.contains(&"profile") {
                Some(user.name())
            } else {
                None
            },
            phone_number: if scopes.contains(&"phone") {
                user.phone.clone()
            } else {
                None
            },
            preferred_username: if scopes.contains(&"profile") {
                Some(user.username.clone())
            } else {
                None
            },
            sub: user.username.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_available_scopes() {
        // All requested scopes are available
        let all_scopes = vec![
            "email".to_string(),
            "profile".to_string(),
            "phone".to_string(),
        ];
        let requested_scopes = vec!["email".to_string(), "profile".to_string()];
        let result = get_available_scopes(&all_scopes, &requested_scopes);
        assert_eq!(result, vec!["email", "profile"]);

        // Some requested scopes are not available
        let all_scopes = vec!["email".to_string(), "profile".to_string()];
        let requested_scopes = vec![
            "email".to_string(),
            "phone".to_string(),
            "profile".to_string(),
        ];
        let result = get_available_scopes(&all_scopes, &requested_scopes);
        assert_eq!(result, vec!["email", "profile"]);

        // No requested scopes
        let all_scopes = vec!["email".to_string(), "profile".to_string()];
        let requested_scopes = vec![];
        let result = get_available_scopes(&all_scopes, &requested_scopes);
        assert_eq!(result, Vec::<&str>::new());

        // No available scopes
        let all_scopes = vec![];
        let requested_scopes = vec!["email".to_string(), "profile".to_string()];
        let result = get_available_scopes(&all_scopes, &requested_scopes);
        assert_eq!(result, Vec::<&str>::new());

        // Both empty
        let all_scopes = vec![];
        let requested_scopes = vec![];
        let result = get_available_scopes(&all_scopes, &requested_scopes);
        assert_eq!(result, Vec::<&str>::new());

        // Duplicate requested scopes
        let all_scopes = vec!["email".to_string(), "profile".to_string()];
        let requested_scopes = vec![
            "email".to_string(),
            "email".to_string(),
            "profile".to_string(),
        ];
        let result = get_available_scopes(&all_scopes, &requested_scopes);
        assert_eq!(result, vec!["email", "email", "profile"]);

        // Case sensitivity
        let all_scopes = vec!["email".to_string(), "profile".to_string()];
        let requested_scopes = vec!["Email".to_string(), "PROFILE".to_string()];
        let result = get_available_scopes(&all_scopes, &requested_scopes);
        assert_eq!(result, Vec::<&str>::new());

        // Single scope match
        let all_scopes = vec!["email".to_string()];
        let requested_scopes = vec!["email".to_string()];
        let result = get_available_scopes(&all_scopes, &requested_scopes);
        assert_eq!(result, vec!["email"]);
    }
}
