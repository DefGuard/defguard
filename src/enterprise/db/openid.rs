use crate::db::{DbPool, User};
use model_derive::Model;
use sqlx::{query_as, Error as SqlxError};

#[derive(Deserialize, Serialize)]
pub struct NewOpenIDClient {
    pub name: String,
    pub redirect_uri: String,
    pub scope: Vec<String>,
    pub enabled: bool,
}

#[derive(Deserialize, Model, PartialEq, Serialize)]
#[table(authorizedapps)]
pub struct AuthorizedApp {
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub user_id: i64,
    pub client_id: String,
    pub home_url: String, // TODO: remove
    pub date: String,     // TODO: NaiveDateTime %d-%m-%Y %H:%M
    pub name: String,
}

impl AuthorizedApp {
    #[must_use]
    pub fn new(
        user_id: i64,
        client_id: String,
        home_url: String,
        date: String,
        name: String,
    ) -> Self {
        Self {
            id: None,
            user_id,
            client_id,
            home_url,
            date,
            name,
        }
    }

    pub async fn find_by_user_and_client_id(
        pool: &DbPool,
        user_id: i64,
        client_id: &str,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", user_id, client_id, home_url, date, name \
            FROM authorizedapps WHERE user_id = $1 AND client_id = $2",
            user_id,
            client_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn all_for_user(pool: &DbPool, user: &User) -> Result<Vec<Self>, SqlxError> {
        if let Some(id) = user.id {
            query_as!(
                Self,
                "SELECT id \"id?\", user_id, client_id, home_url, date, name \
                FROM authorizedapps WHERE user_id = $1",
                id
            )
            .fetch_all(pool)
            .await
        } else {
            Ok(Vec::new())
        }
    }
}
