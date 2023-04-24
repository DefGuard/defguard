use defguard::{
    config::DefGuardConfig,
    db::{init_db, DbPool},
};
use sqlx::{postgres::PgConnectOptions, query, types::Uuid};

pub(super) async fn init_test_db() -> (DbPool, DefGuardConfig) {
    let config = DefGuardConfig::new();
    let opts = PgConnectOptions::new()
        .host(&config.database_host)
        .port(config.database_port)
        .username(&config.database_user)
        .password(&config.database_password)
        .database(&config.database_name);
    let pool = DbPool::connect_with(opts)
        .await
        .expect("Failed to connect to Postgres");
    let db_name = Uuid::new_v4().to_string();
    query(&format!("CREATE DATABASE \"{}\"", db_name))
        .execute(&pool)
        .await
        .expect("Failed to create test database");
    let pool = init_db(
        &config.database_host,
        config.database_port,
        &db_name,
        &config.database_user,
        &config.database_password,
    )
    .await;
    (pool, config)
}

#[allow(dead_code)]
#[cfg(feature = "openid")]
pub(super) static LICENSE_ENTERPRISE: &str = "BwAAAAAAAAB0ZW9uaXRlCgAAAAAAAAAyMDUwLTEwLTEwAAAAAAFiayfBptq8pZXjPo4FV3VnmmwR/ipZHLriVPTW3AFyRq4c2wR+DzWC4BUACu3YMS27kX116JVKWB3/edYKNELFSiqYc6vsfoOrXnnQQJDI8RoyAQB6MpLv/EcgRZh47iI4L+tp44jKFQZ+EqqvMNt3G41u13P72HdkUv8yzQ7dmm3BrYQGJSCh/xiLna+mtQ9IQdqXOmYVInPXiWtIvi157Utfnow3gS0Ak45jci0DhtH+RWmFfiMOQCc4Qx0kEF9PsHl6Hn9Ay4oRTAnSYEPdWfQlVh5Rp276bLqnHDdyJ3/o2RSNK+QUXR7V2iuN1M3sWyW1rCGXtV5miHGI97CS";
#[allow(dead_code)]
pub(super) static LICENSE_EXPIRED: &str = "BwAAAAAAAAB0ZW9uaXRlCgAAAAAAAAAyMDE5LTEwLTEwAAAAAAFuZ7Xm9M20ds/U/PQgVmz4uViCRTJbyAPVLtYRBGvE0i+czH4mxPl4mCyAO1cAOPXNxqh9sAVVr/GzToOix4DfK0aLrYG9FqV5jW13CH+UKTFBqQvN9gGLmnl9+b3pH10gxpGKRZ5fn73fsZsO0SKrJvQ8SAHEQ2+r+VCdZphZ2r9cFR6MC39Ixk4lCki8mz9A4FHZyW4YWWr6k+bxu9RjG/0imh+6OBeddKBpU3HnK96B4rjhiEhrKpfJo6dzib/Mfk+UNZHQA2dAjlocKKxa2+acUaEJQmnaIv4FyFZHl2OzGKkweqDBo0E+Ai7m1g07+pXdXGYb9ykVfoCBEgEX";
#[allow(dead_code)]
#[cfg(feature = "openid")]
pub(super) static LICENSE_WITHOUT_OPENID: &str = "BwAAAAAAAAB0ZW9uaXRlCgAAAAAAAAAyMDUwLTEwLTIyAQABAQCCpzpcqi+8jRX+QTuVjyK0ZmdKa8j+SrA53qSY4rAxjZyt6hgVLlcqTqIbbA7uds5ACa1oBWvQbbPIlGGTpNnG+gQzTm9hAc3CmEd2zMdQXOXzWN8jJHTflsr1dYMxA+tK1el+An+jOY85j0WaRNJma7desF6HEasgEEPktV5P5y3Yh1fULS1scDjEbOJS3pvI07BmSA0/Z+swPMqRzSoyt6NaOUDbR53HR2mMjBSsaZBLsrTQ9Ai16A8fo6pqt2XpSfy/1ImC3mq2q6TG/ABnFw1j65UW0Mx261Bn9184zyLdKPycFUWfyOmOpk/46JZX/PMBHERXeFbmN6YE3KpO";
