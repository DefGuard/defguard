# build release binary
build:
    cargo build --release

# remove test databases
drop-test-dbs:
    ./drop_test_dbs.sh

# move tag to current commit
move-tag TAG:
    # remove local tag
    git tag --delete {{TAG}}
    # remove tag from remote
    git push --delete origin {{TAG}}
    # make new tag
    git tag {{TAG}}
    # push commits to remote
    git push
    # push new tag to remote
    git push origin {{TAG}}

# format Rust project
format:
    cargo +nightly --locked fmt --all  # use nightly toolchain for better import handling

# lint Rust project
lint:
    cargo clippy --all-targets --all-features

# run all migrations
migrate:
    sqlx migrate run

# update sqlx query data
query-data:
    cargo sqlx prepare --workspace -- --all-targets --tests

fix-clippy:
    cargo clippy --all-targets --all-features --fix --allow-dirty -- \
        -W clippy::uninlined_format_args \
        -W clippy::use_self \
        -W clippy::redundant_closure_for_method_calls \
        -W clippy::cloned_instead_of_copied \
        -W clippy::str_to_string \
        -W clippy::explicit_iter_loop

# run LDAP integration tests against a throwaway OpenLDAP container (needs a running Postgres for DATABASE_URL, like other rust tests)
test-ldap *ARGS:
    #!/usr/bin/env bash
    set -uo pipefail
    docker compose -p defguard-ldap -f docker-compose.ldap-test.yaml up -d --wait openldap
    LDAP_URL=ldap://localhost:389 \
    LDAP_BIND_USERNAME=cn=admin,dc=example,dc=org \
    LDAP_BIND_PASSWORD=pass123 \
    LDAP_USER_SEARCH_BASE=ou=users,dc=example,dc=org \
    LDAP_GROUP_SEARCH_BASE=ou=groups,dc=example,dc=org \
    LDAP_USER_CLASS=inetOrgPerson \
    LDAP_GROUP_CLASS=groupOfUniqueNames \
    LDAP_USERNAME_ATTR=cn \
    LDAP_GROUPNAME_ATTR=cn \
    LDAP_MEMBER_ATTR=memberOf \
    LDAP_GROUP_MEMBER_ATTR=uniqueMember \
        cargo nextest run --run-ignored only -E 'package(defguard_core) and test(/^ldap::/)' {{ARGS}}
    status=$?
    docker compose -p defguard-ldap -f docker-compose.ldap-test.yaml down
    exit $status
