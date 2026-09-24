#!/usr/bin/env bash
# /opt/zebra/ops.sh <action> — invoked by the ops GitHub Action over SSM.
set -euo pipefail
cd /opt/zebra; source .env
ACTION="${1:?usage: ops.sh <action>}"

# State is ephemeral and the node has no peers, so genesis is re-injected after
# every start. Idempotent: an already-committed block returns "rejected", HTTP 200.
ecr_login() {
  local reg
  # No ECR image means nothing to log in to; `if` so errexit doesn't abort on it.
  if reg=$(grep -m1 -oE '[0-9]+\.dkr\.ecr\.[a-z0-9-]+\.amazonaws\.com' docker-compose.yml); then
    aws ecr get-login-password --region "${AWS_REGION:-eu-central-1}" \
      | docker login --username AWS --password-stdin "$reg"
  fi
}

self_serve_genesis() {
  local hex out
  hex=$(docker exec zebra-testnet cat /app/testnet-single-node-deploy/genesis.txt | tr -d '[:space:]')
  out=$(curl -s --fail-with-body --retry 30 --retry-delay 2 --retry-connrefused --retry-all-errors \
    http://127.0.0.1:18232 -X POST -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"1.0\",\"id\":\"ops\",\"method\":\"submitblock\",\"params\":[\"$hex\"]}") \
    || { echo "submitblock failed" >&2; return 1; }
  echo "$out"
  case "$out" in
    *'"result":null'*)       echo "genesis: accepted, chain was empty" ;;
    *'"result":"rejected"'*) echo "genesis: already present, nothing to do" ;;
    *)                       echo "genesis: unexpected response" ;;
  esac
}

case "$ACTION" in
  sync)       ecr_login
              # Only zebra's tag is re-pushed; `up -d` pulls a sidecar if its pin moved.
              docker compose pull zebra-testnet
              docker compose up -d
              # logs-api.py is bind-mounted: `up -d` cannot see it change.
              docker compose restart logs-api
              # Only re-up cloudflared if this box already runs one, so a
              # follower never gains a connector from a deploy.
              [ -z "$(docker compose --profile tunnel ps -q cloudflared)" ] \
                || docker compose --profile tunnel up -d cloudflared
              self_serve_genesis ;;
  restart)    docker compose restart zebra-testnet; self_serve_genesis ;;
  start)      docker compose start zebra-testnet; self_serve_genesis ;;
  recreate)   docker compose up -d --force-recreate zebra-testnet; self_serve_genesis ;;
  genesis)    self_serve_genesis ;;
  stop)       docker compose stop zebra-testnet ;;
  logs)       docker compose logs zebra-testnet --tail=50 --no-color ;;
  status)     docker compose ps; bash /opt/zebra/leader.sh status ;;
  # promote/demote are the workflow writing the Role tag; the box only matches it.
  apply)      bash /opt/zebra/leader.sh apply ;;
  *) echo "unknown action: $ACTION"; exit 2 ;;
esac
