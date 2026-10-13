#!/usr/bin/env bash
set -euo pipefail

scenario=$1
network_id=$2
rps=$3

case "$scenario" in
  polling) label='Polling' ;;
  mfa) label='TOTP MFA' ;;
  mfa_posture) label='MFA with posture checks' ;;
  *) echo "::error::Unknown load-test scenario: $scenario"; exit 1 ;;
esac

remote_script=$(cat <<'REMOTE'
set -euo pipefail
exec 9>"$HOME/.defguard-load-test.lock"
flock -n 9 || { echo 'Another load test is already running'; exit 1; }
set -a
source "$HOME/.config/defguard-load-test.env"
set +a
cd "$HOME/defguard"
git fetch origin "$LOAD_TEST_REV"
git checkout --detach "$LOAD_TEST_REV"
git submodule update --init --recursive
cargo build --locked --release -p defguard_load_generator

case "$LOAD_TEST_SCENARIO" in
  polling) command=config-polling; posture=() ;;
  mfa) command=client-mfa; posture=() ;;
  mfa_posture) command=client-mfa; posture=(--with-posture-checks) ;;
  *) echo "Unknown load-test scenario: $LOAD_TEST_SCENARIO"; exit 1 ;;
esac
./target/release/defguard_load_generator test "$command" \
  --proxy-url "$LOAD_TEST_PROXY_URL" \
  --network-id "$NETWORK_ID" \
  --requests-per-second "$RPS" \
  --duration "${DURATION_SECONDS}s" \
  --max-in-flight "$MAX_IN_FLIGHT" \
  "${posture[@]}"
REMOTE
)
encoded_script=$(printf %s "$remote_script" | base64 -w0)
printf -v command 'runuser -u ubuntu -- env LOAD_TEST_REV=%q LOAD_TEST_PROXY_URL=%q LOAD_TEST_SCENARIO=%q NETWORK_ID=%q RPS=%q DURATION_SECONDS=%q MAX_IN_FLIGHT=%q bash -lc %q' "$GITHUB_SHA" "$PROXY_URL" "$scenario" "$network_id" "$rps" "$DURATION_SECONDS" "$MAX_IN_FLIGHT" "printf %s '$encoded_script' | base64 -d | bash"
parameters=$(jq -nc --arg command "$command" '{commands: [$command], executionTimeout: ["2400"]}')
command_id=$(aws ssm send-command --instance-ids "$GENERATOR_INSTANCE_ID" --document-name AWS-RunShellScript --parameters "$parameters" --timeout-seconds 60 --query 'Command.CommandId' --output text)
echo "$label SSM command: $command_id"

status=''
for (( attempt = 0; attempt < 170; attempt++ )); do
  invocation=$(aws ssm get-command-invocation --command-id "$command_id" --instance-id "$GENERATOR_INSTANCE_ID" --output json 2>/dev/null) || { sleep 15; continue; }
  status=$(jq -r '.Status' <<< "$invocation")
  case "$status" in
    Success|Failed|Cancelled|TimedOut) break ;;
  esac
  sleep 15
done
case "$status" in
  Success|Failed|Cancelled|TimedOut) ;;
  *) echo "::error::$label SSM command $command_id did not finish within the workflow deadline"; exit 1 ;;
esac
jq -r '.StandardOutputContent, .StandardErrorContent' <<< "$invocation"
if [[ "$status" != Success || "$(jq -r '.ResponseCode' <<< "$invocation")" != 0 ]]; then
  echo "::error::$label SSM command $command_id finished with status $status"
  exit 1
fi
