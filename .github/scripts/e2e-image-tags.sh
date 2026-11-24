#!/usr/bin/env bash
set -euo pipefail

branch=${GITHUB_REF#refs/heads/}
image_tag=${branch//\//-}
if [[ $branch =~ ^(release|stable)/([0-9]+) ]]; then
  fallback_tag="stable-${BASH_REMATCH[2]}.x"
else
  fallback_tag=dev
fi
env_file=${GITHUB_ENV:-/dev/stdout}

echo "IMAGE_TAG=$image_tag" >> "$env_file"
echo "E2E tests will run on IMAGE_TAG=$image_tag"

# Other repositories build proxy and gateway, so a missing tag falls back to stable or dev.
use_tag_or_fallback() {
  local env_var=$1 image=ghcr.io/defguard/$2
  if ! docker buildx imagetools inspect "$image:$image_tag" > /dev/null 2>&1; then
    echo "::warning::$image:$image_tag not found, E2E tests use $image:$fallback_tag"
    echo "$env_var=$fallback_tag" >> "$env_file"
  fi
}

use_tag_or_fallback EDGE_IMAGE_TAG defguard-proxy
use_tag_or_fallback GATEWAY_IMAGE_TAG gateway
