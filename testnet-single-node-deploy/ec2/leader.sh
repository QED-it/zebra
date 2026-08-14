#!/usr/bin/env bash
# /opt/zebra/leader.sh <elect|promote|demote|status>
#
# The leader is the one instance tagged Role=leader, and the only one running cloudflared.
#
# Claim is an SSM put-parameter WITHOUT --overwrite, which fails if the key
# exists. That is the atomic bit: simultaneous boots cannot both win.
set -euo pipefail
PARAM=/zebra/leader
REGION="${AWS_REGION:-eu-central-1}"

TOKEN=$(curl -sX PUT http://169.254.169.254/latest/api/token \
  -H 'X-aws-ec2-metadata-token-ttl-seconds: 60')
ID=$(curl -s -H "X-aws-ec2-metadata-token: $TOKEN" \
  http://169.254.169.254/latest/meta-data/instance-id)

put() { aws ssm put-parameter --region "$REGION" --name "$PARAM" \
  --type String --value "$ID" "$@" >/dev/null; }
# 0 = value on stdout, 1 = unclaimed, 2 = could not tell
get() {
  local out errf rc
  errf=$(mktemp)
  out=$(aws ssm get-parameter --region "$REGION" --name "$PARAM" \
        --query Parameter.Value --output text 2>"$errf"); rc=$?
  if [ $rc -eq 0 ]; then rm -f "$errf"; printf '%s' "$out"; return 0; fi
  if grep -q ParameterNotFound "$errf"; then rm -f "$errf"; return 1; fi
  echo "leader: cannot read $PARAM: $(cat "$errf")" >&2
  rm -f "$errf"
  return 2
}
# Role=leader exists only on the leader. Followers carry no Role tag at all.
tag()   { aws ec2 create-tags --region "$REGION" --resources "$ID" --tags Key=Role,Value=leader; }
untag() { aws ec2 delete-tags --region "$REGION" --resources "$ID" --tags Key=Role; }

# 0 if we hold the claim afterwards. Takes over from any holder that is not
# actually serving: gone, terminated, or stopped. Safe because elect runs on
# every boot (zebra-leader.service), so a stopped holder that comes back finds
# it has lost and shuts its own cloudflared down.
claim() {
  put 2>/dev/null && return 0
  local cur state rc errf
  cur=$(get); rc=$?
  [ $rc -eq 2 ] && return 1
  [ $rc -eq 1 ] && { put --overwrite; return 0; }
  [ "$cur" = "$ID" ] && return 0
  errf=$(mktemp)
  state=$(aws ec2 describe-instances --region "$REGION" --instance-ids "$cur" \
    --query 'Reservations[].Instances[].State.Name' --output text 2>"$errf") ||
    { grep -q InvalidInstanceID "$errf" || { rm -f "$errf"; return 1; }; state=gone; }
  rm -f "$errf"
  case "$state" in
    ""|None|gone|terminated|shutting-down|stopped|stopping) put --overwrite; return 0 ;;
    *) return 1 ;;
  esac
}

start_tunnel() {
  local t
  t=$(aws ssm get-parameter --region "$REGION" --name /zebra/zebra-testnet/cf-tunnel-token \
    --with-decryption --query Parameter.Value --output text)
  sed -i "s|^CF_TUNNEL_TOKEN=.*|CF_TUNNEL_TOKEN=$t|" /opt/zebra/.env
  cd /opt/zebra && docker compose up -d cloudflared
}

case "${1:-elect}" in
  # Runs at every boot. Losing means stopping our own cloudflared: docker's
  # restart policy would otherwise resurrect it on a box that lost the claim
  # while it was stopped, putting two connectors on the tunnel.
  elect)   if claim; then tag; start_tunnel; echo leader
           else untag 2>/dev/null || true
                cd /opt/zebra && docker compose stop cloudflared >/dev/null 2>&1 || true
                echo follower; fi ;;
  promote) put --overwrite; tag; start_tunnel; echo "leader: $ID" ;;
  # Releases the claim too, so the next elect can take over while this box lives.
  demote)  untag; aws ssm delete-parameter --region "$REGION" --name "$PARAM" 2>/dev/null || true
           cd /opt/zebra && docker compose stop cloudflared; echo "released: $ID" ;;
  status)  cur=$(get) || cur="<unset>"; echo "self=$ID leader=$cur" ;;
  *) echo "usage: leader.sh <elect|promote|demote|status>"; exit 2 ;;
esac
