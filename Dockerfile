FROM rust:latest as chef

WORKDIR /build

# install & cache necessary components
RUN cargo install cargo-chef
RUN rustup component add rustfmt

FROM chef as planner
# prepare recipe
COPY Cargo.toml Cargo.lock ./
COPY src src
COPY model-derive model-derive
COPY proto proto
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
# build deps from recipe & cache as docker layer
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# build project
RUN apt-get update && apt-get -y install protobuf-compiler libprotobuf-dev
COPY Cargo.toml Cargo.lock build.rs sqlx-data.json ./
COPY src src
COPY model-derive model-derive
COPY proto proto
COPY migrations migrations
ENV SQLX_OFFLINE true
RUN cargo install --locked --path . --root /build

FROM node:19-alpine3.15 as web

WORKDIR /app
COPY web/package.json .
COPY web/pnpm-lock.yaml .
COPY web/.npmrc .
RUN npm i -g pnpm@7.19
RUN pnpm install --ignore-scripts
COPY web/ .
RUN pnpm build

# run
FROM debian:bullseye-slim as runtime
RUN apt-get update -y && \
    apt-get install --no-install-recommends -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /build/bin/defguard .
COPY --from=web /app/dist ./web/dist
ENTRYPOINT ["./defguard"]
