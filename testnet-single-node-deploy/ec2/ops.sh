#!/usr/bin/env bash
# /opt/zebra/ops.sh <action> [tag] — invoked by the ops GitHub Action over SSM.
set -euo pipefail
cd /opt/zebra; source .env
ACTION="${1:?usage: ops.sh <action> [tag]}"; TAG="${2:-latest}"

# State is ephemeral and the node has no peers, so genesis is re-injected after
# every start. Idempotent: an already-committed block returns "rejected", HTTP 200.
serve_genesis() {
  local hex
  hex=$(docker exec zebra-testnet cat /app/zebra-test/src/vectors/block-test-0-000-000.txt | tr -d '[:space:]')
  curl -s --retry 30 --retry-delay 2 --retry-connrefused --retry-all-errors \
    http://127.0.0.1:18232 -X POST -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"1.0\",\"id\":\"ops\",\"method\":\"submitblock\",\"params\":[\"$hex\"]}"
  echo
}

case "$ACTION" in
  deploy)     sed -i "s|^IMAGE=.*|IMAGE=$IMAGE_REPO:$TAG|" .env
              docker compose up -d --pull always zebra-testnet
              serve_genesis ;;
  restart)    docker compose restart zebra-testnet; serve_genesis ;;
  start)      docker compose start zebra-testnet; serve_genesis ;;
  recreate)   docker compose up -d --force-recreate zebra-testnet; serve_genesis ;;
  genesis)    serve_genesis ;;
  stop)       docker compose stop zebra-testnet ;;
  logs)       docker compose logs zebra-testnet --tail=200 --no-color ;;
  status)     docker compose ps; bash /opt/zebra/leader.sh status ;;
  promote)    bash /opt/zebra/leader.sh promote ;;
  demote)     bash /opt/zebra/leader.sh demote ;;
  *) echo "unknown action: $ACTION"; exit 2 ;;
esac
