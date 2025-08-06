FROM node:24-alpine AS web

WORKDIR /app
COPY web/package.json web/pnpm-lock.yaml web/.npmrc .
RUN npm i -g pnpm
RUN pnpm install --ignore-scripts --frozen-lockfile
COPY web/ .
RUN pnpm run generate-translation-types
RUN pnpm build

FROM rust:1-alpine AS chef

WORKDIR /build

# install & cache necessary components
RUN apk add musl-dev openssl-dev
RUN cargo install cargo-chef
RUN rustup component add rustfmt

FROM chef AS planner
# prepare recipe
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY proto proto
COPY migrations migrations
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# build deps from recipe & cache as docker layer
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# build project
COPY --from=web /app/dist ./web/dist
COPY web/src/shared/images/svg ./web/src/shared/images/svg
RUN apk add openssl-libs-static protoc protobuf-dev
COPY Cargo.toml Cargo.lock ./
# for vergen
COPY .git .git
COPY .sqlx .sqlx
COPY crates crates
COPY proto proto
COPY migrations migrations
RUN cargo install --locked --bin defguard --path ./crates/defguard --root /build

# run
FROM alpine:3
RUN apk add ca-certificates
WORKDIR /app
COPY --from=builder /build/bin/defguard .
ENTRYPOINT ["./defguard"]
