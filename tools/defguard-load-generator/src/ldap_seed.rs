use std::{collections::HashSet, time::Duration};

use anyhow::Context;
use ldap3::{LdapConnAsync, LdapConnSettings, Scope, SearchEntry, drive};
use secrecy::ExposeSecret;

use crate::config::SeedLdapArgs;

const LOADTEST_OU: &str = "ou=loadtest";
const USERS_OU: &str = "ou=users,ou=loadtest";
const GROUPS_OU: &str = "ou=groups,ou=loadtest";
const GROUP_NAME: &str = "load-test-users";
const USER_PREFIX: &str = "load-test-user-";
const USER_PASSWORD: &str = "userPassword";

pub async fn run(args: SeedLdapArgs) -> anyhow::Result<()> {
    let (conn, mut ldap) = LdapConnAsync::with_settings(
        LdapConnSettings::new().set_conn_timeout(Duration::from_secs(10)),
        &args.ldap_url,
    )
    .await
    .context("failed to connect to LDAP")?;
    drive!(conn);

    ldap.simple_bind(&args.admin_dn, args.admin_password.expose_secret())
        .await?
        .success()
        .context("failed to bind to LDAP")?;

    let base_dn = args.base_dn.trim().to_owned();
    let loadtest_dn = format!("{LOADTEST_OU},{base_dn}");
    let users_dn = format!("{USERS_OU},{base_dn}");
    let groups_dn = format!("{GROUPS_OU},{base_dn}");
    let group_dn = format!("cn={GROUP_NAME},{groups_dn}");

    delete_subtree(&mut ldap, &loadtest_dn).await?;
    add_ou(&mut ldap, &loadtest_dn, "loadtest").await?;
    add_ou(&mut ldap, &users_dn, "users").await?;
    add_ou(&mut ldap, &groups_dn, "groups").await?;

    let mut members = Vec::with_capacity(args.users.get());
    for index in 1..=args.users.get() {
        let uid = format!("{USER_PREFIX}{index:06}");
        let user_dn = format!("uid={uid},{users_dn}");
        add_user(
            &mut ldap,
            &user_dn,
            &uid,
            args.user_password.expose_secret(),
        )
        .await?;
        members.push(user_dn);

        if index == 1 || index % 1000 == 0 || index == args.users.get() {
            tracing::info!(seeded_users = index, "LDAP seed progress");
        }
    }

    let member_values: HashSet<&str> = members.iter().map(String::as_str).collect();
    let group_values = HashSet::from([GROUP_NAME, "load-test-users"]);
    let object_classes = HashSet::from(["top", "groupOfUniqueNames"]);
    let mut group_attrs = vec![("objectClass", object_classes), ("cn", group_values)];
    group_attrs.push(("uniqueMember", member_values));
    ldap.add(&group_dn, group_attrs).await?.success()?;

    tracing::info!(users = args.users.get(), "LDAP seeding complete");
    ldap.unbind().await?;
    Ok(())
}

async fn add_ou(ldap: &mut ldap3::Ldap, dn: &str, name: &str) -> anyhow::Result<()> {
    let object_classes = HashSet::from(["top", "organizationalUnit"]);
    let names = HashSet::from([name]);
    ldap.add(dn, vec![("objectClass", object_classes), ("ou", names)])
        .await?
        .success()?;
    Ok(())
}

async fn add_user(
    ldap: &mut ldap3::Ldap,
    dn: &str,
    uid: &str,
    password: &str,
) -> anyhow::Result<()> {
    let object_classes = HashSet::from(["top", "person", "organizationalPerson", "inetOrgPerson"]);
    let uid_values = HashSet::from([uid]);
    let cn_values = HashSet::from([uid]);
    let sn_values = HashSet::from(["LoadTest"]);
    let given_name_values = HashSet::from(["User"]);
    let password_values = HashSet::from([password]);
    ldap.add(
        dn,
        vec![
            ("objectClass", object_classes),
            ("uid", uid_values),
            ("cn", cn_values),
            ("sn", sn_values),
            ("givenName", given_name_values),
            (USER_PASSWORD, password_values),
        ],
    )
    .await?
    .success()?;
    Ok(())
}

async fn delete_subtree(ldap: &mut ldap3::Ldap, base_dn: &str) -> anyhow::Result<()> {
    let (entries, _) = ldap
        .search(base_dn, Scope::Subtree, "(objectClass=*)", vec!["1.1"])
        .await?
        .success()?;
    let mut dns: Vec<String> = entries
        .into_iter()
        .map(SearchEntry::construct)
        .map(|entry| entry.dn)
        .collect();
    dns.sort_by_key(|dn| std::cmp::Reverse(dn.len()));

    for dn in dns {
        ldap.delete(&dn).await?.success()?;
    }
    Ok(())
}
