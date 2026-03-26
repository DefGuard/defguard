use defguard_common::{
    config::DefGuardConfig,
    db::{
        Id,
        models::{
            Device, DeviceType, User, WireguardNetwork,
            group::{Group, Permission},
            settings::initialize_current_settings,
            wireguard::{LocationMfaMode, ServiceLocationMode},
        },
    },
};
use defguard_core::{
    enterprise::{
        db::models::acl::{AclAlias, AclRule, AliasKind, AliasState, RuleState},
        handlers::acl::{
            ApiAclRule, EditAclRule,
            alias::{ApiAclAlias, EditAclAlias},
            destination::{ApiAclDestination, EditAclDestination},
        },
        license::{get_cached_license, set_cached_license},
    },
    handlers::Auth,
};
use reqwest::StatusCode;
use serde_json::{Value, json};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::net::TcpListener;

use super::common::{
    authenticate_admin, client::TestClient, exceed_enterprise_limits, make_base_client,
    make_test_client, setup_pool,
};
use crate::common::{init_config, initialize_users};

mod aliases;
mod destinations;
mod rules;

async fn make_client_v2(pool: PgPool, config: DefGuardConfig) -> TestClient {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Could not bind ephemeral socket");
    initialize_users(&pool).await;
    initialize_current_settings(&pool)
        .await
        .expect("Could not initialize settings");
    let (client, _) = make_base_client(pool, config, listener).await;
    client
}

fn make_rule() -> EditAclRule {
    EditAclRule {
        name: "rule".to_string(),
        all_locations: false,
        locations: Vec::new(),
        expires: None,
        allow_all_users: false,
        deny_all_users: false,
        allow_all_groups: false,
        deny_all_groups: false,
        allow_all_network_devices: false,
        deny_all_network_devices: false,
        allowed_users: vec![1],
        denied_users: Vec::new(),
        allowed_groups: Vec::new(),
        denied_groups: Vec::new(),
        allowed_network_devices: Vec::new(),
        denied_network_devices: Vec::new(),
        addresses: "10.2.2.2, 10.0.0.1/24, 10.0.10.1-10.0.20.1".to_string(),
        aliases: Vec::new(),
        destinations: Vec::new(),
        enabled: true,
        protocols: vec![6, 17],
        ports: "1, 2, 3, 10-20, 30-40".to_string(),
        any_address: false,
        any_port: false,
        any_protocol: false,
        use_manual_destination_settings: true,
    }
}

async fn set_rule_state(pool: &PgPool, id: Id, state: RuleState, parent_id: Option<Id>) {
    let mut rule = AclRule::find_by_id(pool, id).await.unwrap().unwrap();
    rule.state = state;
    rule.parent_id = parent_id;
    rule.save(pool).await.unwrap();
}

async fn authenticate_promoted_admin(client: &mut TestClient, pool: &PgPool, username: &str) {
    let user = User::find_by_username(pool, username)
        .await
        .unwrap()
        .unwrap();
    let admin_group = Group::find_by_permission(pool, Permission::IsAdmin)
        .await
        .unwrap()
        .into_iter()
        .next()
        .expect("admin group should exist in test database");
    user.add_to_group(pool, &admin_group).await.unwrap();
    client.login_user(username, "pass123").await;
}

fn make_alias() -> EditAclAlias {
    EditAclAlias {
        name: "alias".to_string(),
        addresses: "10.2.2.2, 10.0.0.1/24, 10.0.10.1-10.0.20.1".to_string(),
        protocols: vec![6, 17],
        ports: "1, 2, 3, 10-20, 30-40".to_string(),
    }
}

fn make_destination() -> EditAclDestination {
    EditAclDestination {
        name: "destination".to_string(),
        addresses: "10.20.30.40, 10.0.0.1/24, 10.0.10.1-10.0.20.1".to_string(),
        ports: "1, 2, 3, 10-20, 30-40".to_string(),
        protocols: vec![6, 17],
        any_address: false,
        any_port: false,
        any_protocol: false,
    }
}

async fn count_destinations(pool: &PgPool) -> usize {
    AclAlias::all_of_kind(pool, AliasKind::Destination)
        .await
        .unwrap()
        .len()
}

fn edit_rule_data_into_api_response(
    data: &EditAclRule,
    id: Id,
    parent_id: Option<Id>,
    state: RuleState,
) -> ApiAclRule {
    ApiAclRule {
        id,
        parent_id,
        state,
        name: data.name.clone(),
        all_locations: data.all_locations,
        locations: data.locations.clone(),
        expires: data.expires,
        enabled: data.enabled,
        allow_all_users: data.allow_all_users,
        deny_all_users: data.deny_all_users,
        allow_all_groups: data.allow_all_groups,
        deny_all_groups: data.deny_all_groups,
        allow_all_network_devices: data.allow_all_network_devices,
        deny_all_network_devices: data.deny_all_network_devices,
        allowed_users: data.allowed_users.clone(),
        denied_users: data.denied_users.clone(),
        allowed_groups: data.allowed_groups.clone(),
        denied_groups: data.denied_groups.clone(),
        allowed_network_devices: data.allowed_network_devices.clone(),
        denied_network_devices: data.denied_network_devices.clone(),
        addresses: data.addresses.clone(),
        aliases: data.aliases.clone(),
        destinations: data.destinations.clone(),
        ports: data.ports.clone(),
        protocols: data.protocols.clone(),
        any_address: data.any_address,
        any_port: data.any_port,
        any_protocol: data.any_protocol,
        use_manual_destination_settings: data.use_manual_destination_settings,
    }
}

fn edit_alias_data_into_api_response(
    data: EditAclAlias,
    id: Id,
    parent_id: Option<Id>,
    state: AliasState,
    kind: AliasKind,
    rules: Vec<Id>,
) -> ApiAclAlias {
    ApiAclAlias {
        id,
        parent_id,
        state,
        name: data.name,
        kind,
        addresses: data.addresses,
        ports: data.ports,
        protocols: data.protocols,
        rules,
    }
}

fn edit_destination_data_into_api_response(
    data: EditAclDestination,
    id: Id,
    parent_id: Option<Id>,
    state: AliasState,
    rules: Vec<Id>,
) -> ApiAclDestination {
    ApiAclDestination {
        id,
        parent_id,
        state,
        name: data.name,
        kind: AliasKind::Destination,
        addresses: data.addresses,
        ports: data.ports,
        protocols: data.protocols,
        rules,
        any_address: data.any_address,
        any_port: data.any_port,
        any_protocol: data.any_protocol,
    }
}
