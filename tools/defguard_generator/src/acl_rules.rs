use anyhow::Result;
use defguard_core::enterprise::db::models::acl::{AclRule, RuleState};
use sqlx::{PgPool, query};

pub async fn generate_acl_rules(pool: PgPool, num_rules: u32) -> Result<()> {
    truncate_with_restart(&pool).await?;

    for index in 0..num_rules {
        let acl_rule = AclRule {
            name: format!("Generated {index}"),
            state: RuleState::Applied,
            all_locations: true,
            allow_all_users: true,
            allow_all_groups: true,
            allow_all_network_devices: true,
            ..Default::default()
        };
        acl_rule.save(&pool).await?;
    }

    Ok(())
}

/// Remove all records from sessions and stats tables.
/// This also resets the auto-incrementing sequences.
async fn truncate_with_restart(pool: &PgPool) -> Result<()> {
    query("TRUNCATE aclrule RESTART IDENTITY CASCADE")
        .execute(pool)
        .await?;

    Ok(())
}
