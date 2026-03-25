use anyhow::Result;
use defguard_core::enterprise::db::models::acl::{AclRule, RuleState};
use sqlx::{PgPool, query};

pub async fn generate_acl_rules(pool: PgPool, num_rules: u32) -> Result<()> {
    truncate_with_restart(&pool).await?;

    for index in 0..num_rules {
        let mut acl_rule = AclRule::default();
        acl_rule.name = format!("Generated {index}");
        acl_rule.state = RuleState::Applied;
        acl_rule.all_locations = true;
        acl_rule.allow_all_users = true;
        acl_rule.allow_all_groups = true;
        acl_rule.allow_all_network_devices = true;
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
