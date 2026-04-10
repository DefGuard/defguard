FROM public.ecr.aws/docker/library/node:25 AS web

WORKDIR /app
COPY web/package.json web/pnpm-lock.yaml ./
RUN npm i -g pnpm
RUN pnpm install --ignore-scripts --frozen-lockfile
COPY web/ .
RUN pnpm build

FROM public.ecr.aws/docker/library/rust:1 AS chef

WORKDIR /build

# install & cache necessary components
RUN cargo install cargo-chef
RUN rustup component add rustfmt

FROM chef AS planner
# prepare recipe
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY tools tools
COPY proto proto
COPY migrations migrations
RUN cargo chef prepare --bin defguard --recipe-path recipe.json

FROM chef AS builder
# build deps from recipe & cache as docker layer
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --bin defguard --release --recipe-path recipe.json

# build project
COPY --from=web /app/dist ./web/dist
RUN apt-get update && apt-get -y install protobuf-compiler libprotobuf-dev
COPY Cargo.toml Cargo.lock ./
# for vergen
COPY .git .git
COPY .sqlx .sqlx
COPY crates crates
COPY tools tools
COPY proto proto
COPY migrations migrations
RUN cargo install --locked --bin defguard --path ./crates/defguard --root /build

# run
FROM public.ecr.aws/docker/library/debian:13-slim
# TEMPORARY FIX: The parent image has a snapshot of debian sources that has a security vulnerability. This is a temporary fix until the parent image is updated.
# Remove this once the parent image is updated with the latest debian sources.
RUN sed -i \
        -e 's|snapshot\.debian\.org/archive/debian/[^/]*/|deb.debian.org/debian/|g' \
        -e 's|snapshot\.debian\.org/archive/debian-security/[^/]*/|security.debian.org/debian-security/|g' \
        /etc/apt/sources.list.d/debian.sources && \
    apt-get update -y && apt-get upgrade -y && \
    apt-get install --no-install-recommends -y ca-certificates lsb-release libssl-dev && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /build/bin/defguard .
ENTRYPOINT ["./defguard"]
