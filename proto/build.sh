#!/bin/bash

set -e

log() {
  echo "$@" > /dev/stderr
}

fail() {
  log "$@"
  exit 1
}

THIS_DIR=$( cd "$(dirname "$0")" ; pwd )
PROTOC=/usr/bin/protoc
GRPC_CPP_PLUGIN=/usr/bin/grpc_cpp_plugin

for BIN in PROTOC GRPC_CPP_PLUGIN ; do
  test -x "${!BIN}" || fail "Need ${!BIN}"
done

TMP_DIR=

onExit() {
  [ -z "$TMP_DIR" ] || rm -r "$TMP_DIR"
}

trap onExit EXIT

TMP_DIR=$(mktemp -d)

cd "$THIS_DIR"
find -type f -name \*.proto -print0 \
  | xargs -0 \
    protoc \
      --plugin=protoc-gen-grpc="$GRPC_CPP_PLUGIN" \
      --cpp_out="$TMP_DIR" \
      --grpc_out="$TMP_DIR"

find "$TMP_DIR"
