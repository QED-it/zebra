#!/usr/bin/env bash
# /opt/zebra/leader.sh [apply|status]
#
# Only one instance may run cloudflared. Every process holding the tunnel token
# registers as another connector and Cloudflare load-balances across them, but
# these nodes are not replicas — each has its own ephemeral chain, so two
# connectors means one hostname answering from two different chains.
#
# Role=leader marks the instance allowed to run it. The ops workflow writes that
# tag; this script only makes the box match it. Nothing here elects and nothing
# here writes the tag, so there is no shared state to race over.
set -euo pipefail
cd /opt/zebra
REGION="${AWS_REGION:-eu-central-1}"
TOKEN_PARAM=/zebra/zebra-testnet/cf-tunnel-token
IMDS=http://169.254.169.254/latest

T=$(curl -sf --retry 5 --retry-delay 1 -X PUT "$IMDS/api/token" \
  -H 'X-aws-ec2-metadata-token-ttl-seconds: 60')
ID=$(curl -sf --retry 5 --retry-delay 1 -H "X-aws-ec2-metadata-token: $T" \
  "$IMDS/meta-data/instance-id")
[[ $ID == i-* ]] || { echo "bad instance id: '$ID'" >&2; exit 1; }

# cloudflared sits in a compose profile, so a bare `docker compose up -d` cannot
# start a connector by accident. Always address it through the profile.
dc() { docker compose --profile tunnel "$@"; }
running() { [ -n "$(dc ps -q cloudflared)" ]; }
# Prints the Role tag, or "None". Non-zero means we could not find out, which is
# not the same as "not the leader" — see the hard exit below.
role() { aws ec2 describe-tags --region "$REGION" \
  --filters "Name=resource-id,Values=$ID" "Name=key,Values=Role" \
  --query 'Tags[0].Value' --output text; }

up() {
  if running; then return 0; fi
  local t
  t=$(aws ssm get-parameter --region "$REGION" --name "$TOKEN_PARAM" \
    --with-decryption --query Parameter.Value --output text)
  grep -q '^CF_TUNNEL_TOKEN=' .env || { echo "no CF_TUNNEL_TOKEN= line in .env" >&2; return 1; }
  sed -i "s|^CF_TUNNEL_TOKEN=.*|CF_TUNNEL_TOKEN=$t|" .env
  dc up -d cloudflared
}

ACTION="${1:-apply}"
case $ACTION in apply|status) ;; *) echo "usage: leader.sh [apply|status]" >&2; exit 2 ;; esac

# Fail closed. A throttled or denied lookup must not read as "not the leader",
# which would take the tunnel down over a transient API error.
R=$(role) || { echo "Role tag lookup failed; leaving cloudflared as-is" >&2; exit 1; }

# apply runs at boot and on demand. At boot it also undoes docker's restart
# policy reviving a connector on a box that lost the tag while it was stopped.
if [ "$ACTION" = status ]; then
  echo "self=$ID leader=$([ "$R" = leader ] && echo yes || echo no)" \
       "cloudflared=$(running && echo up || echo down)"
elif [ "$R" = leader ]; then
  up
else
  dc stop cloudflared >/dev/null
fi
