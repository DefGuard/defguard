# Contributing

1. Sqlx offline build

Requires `sqlx-data.json` to be present in the root directory of the project. Create the file using:

```
cargo sqlx prepare -- --lib
```

2. Build docker image

```
docker-compose build
```

3. Run

```
docker-compose up
```

## Configuration

Following environment variables can be set to configure orion core service:

* **DEFGUARD_ADMIN_GROUPNAME**: groupname that give a user privileged access
* **DEFGUARD_DEFAULT_ADMIN_PASSWORD**: initial password of the default `admin` user

### Authorization

* **DEFGUARD_JWT_SECRET**: Json Web Token secret, used to encode/decode JWT tokens

### LDAP

* **DEFGUARD_LDAP_URL**: URL to read users and devices data (e.g. `http://localhost:389`)
* **DEFGUARD_LDAP_GROUP_SEARCH_BASE**: group search base, default: `ou=groups,dc=example,dc=org`
* **DEFGUARD_LDAP_USER_SEARCH_BASE**: user search base, default: `dc=example,dc=org`
* **DEFGUARD_LDAP_USER_OBJ_CLASS**: user object class, default: `inetOrgPerson`
* **DEFGUARD_LDAP_GROUP_OBJ_CLASS**: group object class, default: `groupOfUniqueNames`
* **DEFGUARD_LDAP_USERNAME_ATTR**: naming attribute for users, should be `cn` or `uid`, default: `cn`
* **DEFGUARD_LDAP_GROUPNAME_ATTR**: naming attribute for groups, default: `cn`
* **DEFGUARD_LDAP_MEMBER_ATTR**: user attribute for group membership
* **DEFGUARD_LDAP_GROUP_MEMBER_ATTR**: group attibute for memebers

### gRPC

* **DEFGUARD_GRPC_PORT**: gRPC services bind port, default = `50055`

### HTTP server

* **DEFGUARD_WEB_PORT**: web services bind port, default = `8000`
