use defguard_common::db::{
    Id,
    models::{User, WireguardNetwork},
};

use crate::enterprise::db::models::acl::AclRuleInfo;

pub async fn get_allowed_ips_from_acl_rules(location: &WireguardNetwork<Id>, user: User<Id>) {
    // fetch all rules assigned to location
    let acl_rules: Vec<AclRuleInfo> = todo!();

    // fetch all groups the user is a member of
    let groups = todo!();

    for rule in acl_rules {
        // determine effective allowed users & groups
        let effective_allowed_users = todo!();
        let effective_allowed_groups = todo!();

        // check if user matches one of those users or groups

        // if the rule matches the user fetch all destination addresses and add to result
    }

    // merge destinations into smallest possible list of non-overlapping subnets
}
