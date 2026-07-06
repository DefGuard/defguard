# default Docker image tag for e2e tests
IMAGE_TAG := "stable-1.x"

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

# run e2e tests (requires Docker, IMAGE_TAG defaults to stable-1.x)
e2e-test *ARGS='':
    cd e2e && IMAGE_TAG="{{IMAGE_TAG}}" pnpm exec playwright test {{ARGS}}

# update sqlx query data
query-data:
    cargo sqlx prepare --workspace -- --all-targets --tests
