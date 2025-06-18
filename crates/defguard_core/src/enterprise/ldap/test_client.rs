use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    vec::Vec,
};

use ldap3::{Mod, SearchEntry};

use super::error::LdapError;
use crate::{
    db::{Group, User},
    enterprise::ldap::model::extract_rdn_value,
};

/// Extract attribute value from LDAP filter
///
/// This function handles both simple attribute=value patterns and compound filters like
/// "(&(cn=testuser)(objectClass=inetOrgPerson))".
fn extract_attribute_value(filter: &str, attr: &str) -> Option<String> {
    let filter = filter.trim().trim_start_matches('(').trim_end_matches(')');

    if filter.starts_with('&') {
        let inner = filter
            .trim_start_matches('&')
            .trim_start_matches('(')
            .trim_end_matches(')');
        for condition in inner.split(")(") {
            if let Some(value) = extract_simple_attribute_value(condition, attr) {
                return Some(value);
            }
        }
    } else {
        return extract_simple_attribute_value(filter, attr);
    }

    None
}

/// Extract value from simple attribute=value pattern
fn extract_simple_attribute_value(condition: &str, attr: &str) -> Option<String> {
    if condition.starts_with(attr) && condition.contains('=') {
        let parts: Vec<&str> = condition.splitn(2, '=').collect();
        if parts.len() == 2 && parts[0] == attr {
            return Some(parts[1].to_string());
        }
    }
    None
}

#[derive(Debug, Clone)]
pub(super) enum LdapEvent {
    ObjectAdded {
        dn: String,
        attrs: Vec<(String, HashSet<String>)>,
    },
    ObjectModified {
        old_dn: String,
        new_dn: String,
        mods: Vec<Mod<String>>,
    },
    ObjectDeleted {
        dn: String,
    },
    UserBound {
        dn: String,
        password: String,
    },
}

/// Compare two vectors for equality, ignoring order of elements.
fn vecs_equal_unordered<T>(vec1: &[T], vec2: &[T]) -> bool
where
    T: PartialEq + Clone,
{
    if vec1.len() != vec2.len() {
        false
    } else {
        let mut vec2_copy = vec2.to_vec();
        vec1.iter().all(|item| {
            if let Some(pos) = vec2_copy.iter().position(|x| x == item) {
                vec2_copy.remove(pos);
                true
            } else {
                false
            }
        })
    }
}

// This must be implemented by hand to ignore order in attributes and modifications.
impl PartialEq for LdapEvent {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                LdapEvent::ObjectAdded { dn, attrs },
                LdapEvent::ObjectAdded {
                    dn: other_dn,
                    attrs: other_attrs,
                },
            ) => dn == other_dn && vecs_equal_unordered(attrs, other_attrs),
            (
                LdapEvent::ObjectModified {
                    old_dn,
                    new_dn,
                    mods,
                },
                LdapEvent::ObjectModified {
                    old_dn: other_old_dn,
                    new_dn: other_new_dn,
                    mods: other_mods,
                },
            ) => {
                old_dn == other_old_dn
                    && new_dn == other_new_dn
                    && vecs_equal_unordered(mods, other_mods)
            }
            (LdapEvent::ObjectDeleted { dn }, LdapEvent::ObjectDeleted { dn: other_dn }) => {
                dn == other_dn
            }
            (
                LdapEvent::UserBound { dn, password },
                LdapEvent::UserBound {
                    dn: other_dn,
                    password: other_password,
                },
            ) => dn == other_dn && password == other_password,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Object {
    User(User),
    Group(Group),
}

impl Object {
    fn to_search_entry(&self, dn: &str, config: &super::LDAPConfig) -> SearchEntry {
        match self {
            Object::User(user) => SearchEntry {
                dn: dn.to_string(),
                attrs: user
                    .to_test_attrs(None, config)
                    .into_iter()
                    .map(|(k, v)| (k, v.into_iter().collect()))
                    .collect(),
                bin_attrs: HashMap::new(),
            },
            Object::Group(group) => SearchEntry {
                dn: dn.to_string(),
                attrs: group
                    .to_test_attrs(config, None)
                    .into_iter()
                    .map(|(k, v)| (k, v.into_iter().collect()))
                    .collect(),
                bin_attrs: HashMap::new(),
            },
        }
    }
}

/// A test client for simulating LDAP operations in tests.
/// It stores events, objects, and group memberships to verify behavior without a real LDAP server.
///
/// LDAP operations don't actually modify any data, but emit corresponding events that may
/// be verified using the `events_match` method.
///
/// To modify (setup) the mock data, use the `add_test_user`, `add_test_group`, and `add_test_membership` methods.
#[derive(Debug, Default)]
pub struct TestClient {
    events: Vec<LdapEvent>,
    // DN: Object
    pub(super) objects: HashMap<String, Object>,
    // DN: DN
    pub(super) memberships: HashMap<String, HashSet<String>>,
}

impl TestClient {
    fn add_event(&mut self, event: LdapEvent) {
        self.events.push(event);
    }

    pub(super) fn events_match(&self, expected: &[LdapEvent], order_matters: bool) -> bool {
        if self.events.len() != expected.len() {
            return false;
        }
        if order_matters {
            self.events == expected
        } else {
            vecs_equal_unordered(&self.events, expected)
        }
    }

    pub(super) fn clear_events(&mut self) {
        self.events.clear();
    }

    pub(super) fn add_test_user(&mut self, user: &User, config: &super::LDAPConfig) {
        let dn = config.user_dn_from_user(user);
        self.objects.insert(dn, Object::User(user.clone()));
    }

    pub(super) fn remove_test_user(&mut self, user: &User, config: &super::LDAPConfig) {
        let dn = config.user_dn_from_user(user);
        self.objects.remove(&dn);
    }

    pub(super) fn add_test_group(&mut self, group: &Group, config: &super::LDAPConfig) {
        let dn = config.group_dn(&group.name);
        self.objects.insert(dn, Object::Group(group.clone()));
    }

    pub(super) fn add_test_membership(
        &mut self,
        group: &Group,
        user: &User,
        config: &super::LDAPConfig,
    ) {
        let group_dn = config.group_dn(&group.name);
        let user_dn = config.user_dn_from_user(user);
        self.memberships
            .entry(group_dn)
            .or_default()
            .insert(user_dn);
    }

    pub(super) fn remove_test_membership(
        &mut self,
        group: &Group,
        user: &User,
        config: &super::LDAPConfig,
    ) {
        let group_dn = config.group_dn(&group.name);
        let user_dn = config.user_dn_from_user(user);
        if let Some(members) = self.memberships.get_mut(&group_dn) {
            members.remove(&user_dn);
            if members.is_empty() {
                self.memberships.remove(&group_dn);
            }
        }
    }

    pub(super) fn get_events(&self) -> &[LdapEvent] {
        &self.events
    }
}

impl super::LDAPConnection {
    pub(crate) async fn create() -> Result<super::LDAPConnection, LdapError> {
        Ok(Self {
            config: super::LDAPConfig::default(),
            url: String::new(),
            test_client: TestClient::default(),
        })
    }

    pub(super) async fn search_users(
        &mut self,
        filter: &str,
    ) -> Result<Vec<SearchEntry>, LdapError> {
        let rdn_attr = self
            .config
            .ldap_user_rdn_attr
            .clone()
            .unwrap_or(self.config.ldap_username_attr.clone());
        let username_attr = self.config.ldap_username_attr.clone();

        let mut results = Vec::new();

        let search_value = extract_attribute_value(filter, &username_attr)
            .map(|value| (username_attr.clone(), value))
            .or_else(|| {
                extract_attribute_value(filter, &rdn_attr).map(|value| (rdn_attr.clone(), value))
            });

        if let Some((attr, value)) = search_value {
            for (dn, object) in &self.test_client.objects {
                if let Object::User(user) = object {
                    let matches = if attr == username_attr {
                        user.username == value
                    } else if attr == rdn_attr {
                        let rdn_value = if rdn_attr == username_attr {
                            &user.username
                        } else {
                            dn.split(',')
                                .next()
                                .and_then(|rdn_part| rdn_part.split('=').nth(1))
                                .unwrap_or(&user.username)
                        };
                        rdn_value == value
                    } else {
                        false
                    };

                    if matches {
                        results.push(object.to_search_entry(dn, &self.config));
                    }
                }
            }
        } else {
            for (dn, object) in &self.test_client.objects {
                if let Object::User(_) = object {
                    results.push(object.to_search_entry(dn, &self.config));
                }
            }
        }

        Ok(results)
    }

    pub(super) async fn search_groups(
        &mut self,
        filter: &str,
    ) -> Result<Vec<SearchEntry>, LdapError> {
        let groupname = extract_attribute_value(filter, &self.config.ldap_groupname_attr).unwrap();
        let group_dns = self
            .test_client
            .memberships
            .iter()
            .filter_map(|(group_dn, members)| {
                let name = extract_rdn_value(group_dn).unwrap();
                if name == groupname {
                    Some((group_dn.clone(), members.clone()))
                } else {
                    None
                }
            })
            .collect::<HashMap<_, _>>();

        let mut groups = Vec::new();
        for (group_dn, _) in group_dns {
            if let Some(group_object) = self.test_client.objects.get(&group_dn) {
                groups.push(group_object.to_search_entry(&group_dn, &self.config));
            }
        }

        Ok(groups)
    }

    pub(super) async fn test_bind_user(
        &mut self,
        dn: &str,
        password: &str,
    ) -> Result<(), LdapError> {
        self.test_client.add_event(LdapEvent::UserBound {
            dn: dn.to_string(),
            password: password.to_string(),
        });
        Ok(())
    }

    pub(super) async fn get_user_groups(
        &mut self,
        user_dn: &str,
    ) -> Result<Vec<SearchEntry>, LdapError> {
        if let Some(Object::User(_)) = self.test_client.objects.get(user_dn) {
            let group_dns = self
                .test_client
                .memberships
                .iter()
                .filter_map(|(group_dn, members)| {
                    if members.contains(user_dn) {
                        Some(group_dn)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            let mut groups = Vec::new();
            for group_dn in group_dns {
                if let Some(group_object) = self.test_client.objects.get(group_dn) {
                    groups.push(group_object.to_search_entry(group_dn, &self.config));
                }
            }

            return Ok(groups);
        }
        Ok(vec![])
    }

    pub(super) async fn add(
        &mut self,
        dn: &str,
        attrs: Vec<(&str, HashSet<&str>)>,
    ) -> Result<(), LdapError> {
        self.test_client.add_event(LdapEvent::ObjectAdded {
            dn: dn.to_string(),
            attrs: attrs
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
                .collect(),
        });
        Ok(())
    }

    pub(super) async fn modify<S>(
        &mut self,
        old_dn: &str,
        new_dn: &str,
        mods: Vec<Mod<S>>,
    ) -> Result<(), LdapError>
    where
        S: AsRef<[u8]> + Eq + Hash,
    {
        let to_string = |s: S| String::from_utf8(s.as_ref().to_vec()).unwrap();
        let mods = mods
            .into_iter()
            .map(|modification| match modification {
                Mod::Add(attr, set) => {
                    Mod::Add(to_string(attr), set.into_iter().map(to_string).collect())
                }
                Mod::Replace(attr, set) => {
                    Mod::Replace(to_string(attr), set.into_iter().map(to_string).collect())
                }
                Mod::Delete(attr, set) => {
                    Mod::Delete(to_string(attr), set.into_iter().map(to_string).collect())
                }
                Mod::Increment(attr, value) => Mod::Increment(to_string(attr), to_string(value)),
            })
            .collect();

        self.test_client.add_event(LdapEvent::ObjectModified {
            old_dn: old_dn.to_string(),
            new_dn: new_dn.to_string(),
            mods,
        });
        Ok(())
    }

    pub(super) async fn delete(&mut self, dn: &str) -> Result<(), LdapError> {
        self.test_client
            .add_event(LdapEvent::ObjectDeleted { dn: dn.to_string() });
        Ok(())
    }

    pub(super) async fn get_ldap_group_memberships<'a>(
        &mut self,
        all_ldap_users: &'a [User],
    ) -> Result<HashMap<String, HashSet<&'a User>>, LdapError> {
        let memberships = self.test_client.memberships.clone();
        let mut result = HashMap::new();
        let user_dns = all_ldap_users
            .iter()
            .map(|user| self.config.user_dn_from_user(user))
            .collect::<HashSet<_>>();
        for (group_dn, member_dns) in memberships {
            let members = member_dns
                .iter()
                .filter_map(|member_dn| {
                    if user_dns.contains(member_dn) {
                        all_ldap_users
                            .iter()
                            .find(|user| self.config.user_dn_from_user(user) == *member_dn)
                    } else {
                        None
                    }
                })
                .collect::<HashSet<_>>();
            let group_name = extract_rdn_value(&group_dn).unwrap();
            result.insert(group_name, members);
        }
        Ok(result)
    }

    pub(crate) async fn get_group_members(
        &mut self,
        groupname: &str,
    ) -> Result<Vec<String>, LdapError> {
        for (group_dn, members) in &self.test_client.memberships {
            if extract_rdn_value(group_dn).unwrap() == groupname {
                return Ok(members.iter().cloned().collect());
            }
        }

        panic!("Group not found: {}", groupname);
    }

    pub(super) async fn list_users(&mut self) -> Result<Vec<SearchEntry>, LdapError> {
        let mut users = Vec::new();
        let config = &self.config;
        let mut classes = config.ldap_user_auxiliary_obj_classes.clone();
        classes.push(config.ldap_user_obj_class.clone());
        for (dn, object) in &self.test_client.objects {
            if let Object::User(user) = object {
                let rdn_attr = config
                    .ldap_user_rdn_attr
                    .clone()
                    .unwrap_or(config.ldap_username_attr.clone());
                let attrs = user.as_ldap_attrs(
                    "",
                    "",
                    classes.iter().map(|s| s.as_str()).collect(),
                    false,
                    &config.ldap_username_attr,
                    &rdn_attr,
                );
                users.push(SearchEntry {
                    dn: dn.clone(),
                    attrs: attrs
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
                        .collect(),
                    bin_attrs: HashMap::new(),
                });
            }
        }
        Ok(users)
    }

    pub(super) async fn is_member_of(
        &mut self,
        user_dn: &str,
        groupname: &str,
    ) -> Result<bool, LdapError> {
        for (group_dn, members) in &self.test_client.memberships {
            if extract_rdn_value(group_dn).unwrap() == groupname {
                return Ok(members.contains(user_dn));
            }
        }

        Ok(false)
    }

    pub(super) async fn get(&mut self, dn: &str) -> Result<Option<SearchEntry>, LdapError> {
        if let Some(object) = self.test_client.objects.get(dn) {
            let search_entry = object.to_search_entry(dn, &self.config);
            Ok(Some(search_entry))
        } else {
            Ok(None)
        }
    }

    #[cfg(test)]
    pub(super) fn test_client_mut(&mut self) -> &mut TestClient {
        &mut self.test_client
    }
}

#[cfg(test)]
impl<I> User<I> {
    pub(super) fn to_test_attrs(
        &self,
        password: Option<&str>,
        config: &super::LDAPConfig,
    ) -> Vec<(String, HashSet<String>)> {
        let rdn_attr = config
            .ldap_user_rdn_attr
            .clone()
            .unwrap_or(config.ldap_username_attr.clone());
        let classes = config.get_all_user_obj_classes();
        let ssha_password = if let Some(password) = &password {
            super::hash::salted_sha1_hash(password)
        } else {
            String::new()
        };
        let nt_password = if let Some(password) = &password {
            super::hash::nthash(password)
        } else {
            String::new()
        };
        self.as_ldap_attrs(
            &ssha_password,
            &nt_password,
            classes.iter().map(|s| s.as_str()).collect(),
            false,
            &config.ldap_username_attr,
            &rdn_attr,
        )
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
        .collect()
    }
}

#[cfg(test)]
impl<I> Group<I> {
    pub(super) fn to_test_attrs(
        &self,
        config: &super::LDAPConfig,
        members: Option<&Vec<&User<I>>>,
    ) -> Vec<(String, HashSet<String>)> {
        use crate::hashset;

        let mut attrs = vec![
            (
                config.ldap_groupname_attr.clone(),
                hashset![self.name.clone()],
            ),
            (
                "objectClass".to_string(),
                hashset![config.ldap_group_obj_class.clone()],
            ),
        ];

        if let Some(members) = members {
            for user in members {
                let user_dn = config.user_dn_from_user(user);
                attrs.push((config.ldap_group_member_attr.clone(), hashset![user_dn]));
            }
        }

        attrs
    }
}

#[cfg(test)]
mod tests {
    use crate::db::User;

    #[tokio::test]
    async fn test_search_users_by_username() {
        let mut ldap_conn = super::super::LDAPConnection::create().await.unwrap();
        let config = ldap_conn.config.clone();

        let test_user = User::new(
            "testuser",
            Some("hash"),
            "User",
            "Test",
            "test@example.com",
            None,
        );

        let test_dn = "cn=testuser,ou=users,dc=example,dc=com";
        ldap_conn
            .test_client_mut()
            .add_test_user(&test_user, &config);

        let filter = "(cn=testuser)";
        let results = ldap_conn.search_users(filter).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].dn, test_dn);
    }

    #[tokio::test]
    async fn test_search_users_compound_filter() {
        let mut ldap_conn = super::super::LDAPConnection::create().await.unwrap();
        let config = ldap_conn.config.clone();

        let test_user = User::new(
            "test.user",
            Some("hash"),
            "Test",
            "User",
            "test.user@example.com",
            None,
        );

        let test_dn = "cn=test.user,ou=users,dc=example,dc=com";
        ldap_conn
            .test_client_mut()
            .add_test_user(&test_user, &config);

        let filter = "(&(cn=test.user)(objectClass=inetOrgPerson))";
        let results = ldap_conn.search_users(filter).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].dn, test_dn);
    }

    #[tokio::test]
    async fn test_search_users_no_match() {
        let mut ldap_conn = super::super::LDAPConnection::create().await.unwrap();
        let config = ldap_conn.config.clone();

        let test_user = User::new(
            "test.user",
            Some("hash"),
            "Test",
            "User",
            "test.user@example.com",
            None,
        );

        ldap_conn
            .test_client_mut()
            .add_test_user(&test_user, &config);

        let filter = "(cn=nonexistent)";
        let results = ldap_conn.search_users(filter).await.unwrap();

        assert_eq!(results.len(), 0);
    }
}
