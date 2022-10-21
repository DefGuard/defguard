use crate::DbPool;
use model_derive::Model;
use sqlx::{query_as, query_scalar, Error as SqlxError};

#[derive(Model)]
pub struct Group {
    pub(crate) id: Option<i64>,
    pub name: String,
}

impl Group {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            id: None,
            name: name.into(),
        }
    }

    pub async fn find_by_name(pool: &DbPool, name: &str) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", name FROM \"group\" WHERE name = $1",
            name
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn member_usernames(&self, pool: &DbPool) -> Result<Vec<String>, SqlxError> {
        if let Some(id) = self.id {
            query_scalar!(
                "SELECT \"user\".username FROM \"user\" JOIN group_user ON \"user\".id = group_user.user_id \
                WHERE group_user.group_id = $1",
                id
            )
            .fetch_all(pool)
            .await
        } else {
            Ok(Vec::new())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::User;

    #[sqlx::test]
    async fn test_group(pool: DbPool) {
        let mut group = Group::new("worker");
        group.save(&pool).await.unwrap();

        let fetched_group = Group::find_by_name(&pool, "worker").await.unwrap();
        assert!(fetched_group.is_some());
        assert_eq!(fetched_group.unwrap().name, "worker");

        let fetched_group = Group::find_by_name(&pool, "wheel").await.unwrap();
        assert!(fetched_group.is_none());

        group.delete(&pool).await.unwrap();

        let fetched_group = Group::find_by_name(&pool, "worker").await.unwrap();
        assert!(fetched_group.is_none());
    }

    #[sqlx::test]
    async fn test_group_members(pool: DbPool) {
        let mut group = Group::new("worker");
        group.save(&pool).await.unwrap();

        let mut user = User::new(
            "hpotter".into(),
            "pass123",
            "Potter".into(),
            "Harry".into(),
            "h.potter@hogwart.edu.uk".into(),
            None,
        );
        user.save(&pool).await.unwrap();
        user.add_to_group(&pool, &group).await.unwrap();

        let members = group.member_usernames(&pool).await.unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0], user.username);

        user.remove_from_group(&pool, &group).await.unwrap();

        let members = group.member_usernames(&pool).await.unwrap();
        assert!(members.is_empty());
    }
}
