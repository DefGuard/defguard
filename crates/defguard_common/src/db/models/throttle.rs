use chrono::{TimeDelta, Utc};
use sqlx::{PgExecutor, PgPool, Type, query, query_scalar};
use tracing::debug;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum ThrottleScope {
    /// Web and setup login steps, keyed by username.
    WebLogin,
    /// VPN MFA TOTP and email codes, keyed by `<location_id>:<device_id>`.
    VpnMfaCode,
    /// VPN MFA step initiations, which can send email codes, keyed like `VpnMfaCode`.
    VpnMfaInitiate,
}

impl ThrottleScope {
    #[must_use]
    pub const fn limit(self) -> i32 {
        match self {
            Self::WebLogin => 5,
            Self::VpnMfaCode => 10,
            Self::VpnMfaInitiate => 20,
        }
    }

    const fn window(self) -> TimeDelta {
        match self {
            Self::WebLogin => TimeDelta::minutes(5),
            Self::VpnMfaCode | Self::VpnMfaInitiate => TimeDelta::minutes(15),
        }
    }

    /// Charge one attempt for `key`, returning `false` once the window's limit is used up.
    ///
    /// Call it before the credential is examined, and [`Self::refund`] a correct one.
    pub async fn hit<'e, E: PgExecutor<'e>>(self, executor: E, key: &str) -> sqlx::Result<bool> {
        let now = Utc::now().naive_utc();
        let expires_at = now + self.window();
        let attempts = query_scalar!(
            "INSERT INTO throttle AS t (scope, key, attempts, expires_at) \
             VALUES ($1, $2, 1, $4) \
             ON CONFLICT (scope, key) DO UPDATE SET \
                attempts = CASE WHEN t.expires_at <= $3 THEN 1 ELSE t.attempts + 1 END, \
                expires_at = CASE WHEN t.expires_at <= $3 THEN $4 ELSE t.expires_at END \
             RETURNING attempts",
            self as ThrottleScope,
            key,
            now,
            expires_at,
        )
        .fetch_one(executor)
        .await?;

        Ok(attempts <= self.limit())
    }

    pub async fn refund<'e, E: PgExecutor<'e>>(self, executor: E, key: &str) -> sqlx::Result<()> {
        query!(
            "UPDATE throttle SET attempts = attempts - 1 \
             WHERE scope = $1 AND key = $2 AND attempts > 0",
            self as ThrottleScope,
            key,
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    /// Whether the next [`Self::hit`] for `key` returns `false`.
    pub async fn is_blocked<'e, E: PgExecutor<'e>>(
        self,
        executor: E,
        key: &str,
    ) -> sqlx::Result<bool> {
        let attempts = query_scalar!(
            "SELECT attempts FROM throttle WHERE scope = $1 AND key = $2 AND expires_at > $3",
            self as ThrottleScope,
            key,
            Utc::now().naive_utc(),
        )
        .fetch_optional(executor)
        .await?;

        Ok(attempts.is_some_and(|attempts| attempts >= self.limit()))
    }
}

pub async fn reap_expired(pool: &PgPool) -> sqlx::Result<u64> {
    let result = query!("DELETE FROM throttle WHERE expires_at < (now() AT TIME ZONE 'UTC')")
        .execute(pool)
        .await?;
    let count = result.rows_affected();
    debug!("Reaped {count} expired throttle row(s)");
    Ok(count)
}

#[cfg(any(test, feature = "test-support"))]
impl ThrottleScope {
    pub async fn attempts(self, pool: &PgPool, key: &str) -> Option<i32> {
        query_scalar!(
            "SELECT attempts FROM throttle WHERE scope = $1 AND key = $2",
            self as ThrottleScope,
            key,
        )
        .fetch_optional(pool)
        .await
        .unwrap()
    }

    pub async fn end_window(self, pool: &PgPool, key: &str) {
        query!(
            "UPDATE throttle SET expires_at = expires_at - interval '1 day' \
             WHERE scope = $1 AND key = $2",
            self as ThrottleScope,
            key,
        )
        .execute(pool)
        .await
        .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use tokio::task::JoinSet;

    use super::*;
    use crate::db::setup_pool;

    #[sqlx::test]
    async fn test_throttle_limits_failures_per_window(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let scope = ThrottleScope::WebLogin;
        let limit = scope.limit();

        for _ in 0..limit {
            assert!(scope.hit(&pool, "alice").await.unwrap());
        }
        assert!(scope.is_blocked(&pool, "alice").await.unwrap());

        scope.refund(&pool, "alice").await.unwrap();
        assert!(!scope.is_blocked(&pool, "alice").await.unwrap());
        assert!(scope.hit(&pool, "alice").await.unwrap());
        assert!(!scope.hit(&pool, "alice").await.unwrap());

        assert!(!scope.is_blocked(&pool, "bob").await.unwrap());
        assert!(ThrottleScope::VpnMfaCode.hit(&pool, "alice").await.unwrap());
        assert_eq!(
            ThrottleScope::VpnMfaCode.attempts(&pool, "alice").await,
            Some(1)
        );

        scope.end_window(&pool, "alice").await;
        assert!(!scope.is_blocked(&pool, "alice").await.unwrap());
        assert!(scope.hit(&pool, "alice").await.unwrap());
        assert_eq!(scope.attempts(&pool, "alice").await, Some(1));

        scope.end_window(&pool, "alice").await;
        assert_eq!(reap_expired(&pool).await.unwrap(), 1);
        assert_eq!(scope.attempts(&pool, "alice").await, None);
        assert_eq!(
            ThrottleScope::VpnMfaCode.attempts(&pool, "alice").await,
            Some(1)
        );
    }

    #[sqlx::test]
    async fn test_throttle_concurrent_hits_cannot_pass_limit(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let scope = ThrottleScope::VpnMfaCode;

        let mut tasks = JoinSet::new();
        for _ in 0..2 * scope.limit() {
            let pool = pool.clone();
            tasks.spawn(async move { scope.hit(&pool, "1:1").await.unwrap() });
        }
        let allowed = tasks
            .join_all()
            .await
            .into_iter()
            .filter(|allowed| *allowed)
            .count();

        assert_eq!(allowed, scope.limit() as usize);
        assert_eq!(scope.attempts(&pool, "1:1").await, Some(2 * scope.limit()));
    }
}
