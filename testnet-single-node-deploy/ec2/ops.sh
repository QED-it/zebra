#!/usr/bin/env bash
# /opt/zebra/ops.sh <action> [tag] — invoked by the ops GitHub Action over SSM.
set -euo pipefail
cd /opt/zebra; source .env
ACTION="${1:?usage: ops.sh <action> [tag]}"; TAG="${2:-latest}"

# State is ephemeral and the node has no peers, so genesis is re-injected after
# every start. Idempotent: an already-committed block returns "rejected", HTTP 200.
self_serve_genesis() {
  local hex
  hex=$(docker exec zebra-testnet cat /app/zebra-test/src/vectors/block-test-0-000-000.txt | tr -d '[:space:]')
  curl -s --fail-with-body --retry 30 --retry-delay 2 --retry-connrefused --retry-all-errors \
    http://127.0.0.1:18232 -X POST -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"1.0\",\"id\":\"ops\",\"method\":\"submitblock\",\"params\":[\"$hex\"]}" \
    || { echo "submitblock failed" >&2; return 1; }
  echo
}

case "$ACTION" in
  deploy)     grep -q '^IMAGE=' .env || { echo "no IMAGE= line in .env" >&2; exit 1; }
              case "$IMAGE_REPO" in
                *.dkr.ecr.*.amazonaws.com/*)
                  aws ecr get-login-password --region "${AWS_REGION:-eu-central-1}" \
                    | docker login --username AWS --password-stdin "${IMAGE_REPO%%/*}" ;;
              esac
              docker pull "$IMAGE_REPO:$TAG"
              sed -i "s|^IMAGE=.*|IMAGE=$IMAGE_REPO:$TAG|" .env
              docker compose up -d zebra-testnet
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
