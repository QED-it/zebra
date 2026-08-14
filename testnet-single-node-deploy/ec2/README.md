# ZSATestnet on EC2

Four containers on one box running `../testnet-config.toml` (ZSATestnet, magic
`5a534131`, NU5/6/7 at height 1, no PoW, no peers). The launch template was
created manually in the console; the box is driven by GitHub Actions over SSM.

| File | On the box |
| --- | --- |
| `docker-compose.yml` | `/opt/zebra/` |
| `ops.sh`, `leader.sh`, `logs-api.py` | `/opt/zebra/` |
| `zebra-leader.service` | `/etc/systemd/system/` |

zebrad reads the config baked into the image; override single keys with
`ZEBRA_SECTION__KEY` env vars in compose.

| URL | |
| --- | --- |
| `rpc.test-zsa.org` | JSON-RPC, POST only |
| `logs.test-zsa.org` | JSON logs, `?limit=N` (max 500) |
| `dozzle.test-zsa.org` | log UI |

All public and unauthenticated, and 18232/18233/8080 are open on the instance IP
too. `enable_cookie_auth = false`, so anyone reaching 18232 can `stop` or
`invalidateblock`. Disposable testnet only.

## Instance tags and leader election

A Cloudflare Tunnel has one token, and every `cloudflared` holding it registers
as another connector — Cloudflare then round-robins the public hostnames across
them. So exactly one instance runs `cloudflared`: the one tagged `Role=leader`.
**Only the leader carries that tag**; every other instance has no `Role` tag.

`zebra-leader.service` runs `leader.sh elect` on **every** boot:

```
put-parameter /zebra/leader = <own-instance-id>   (no --overwrite)

  success                            -> tag Role=leader, start cloudflared
  ParameterAlreadyExists
    holder gone/terminated/stopped   -> overwrite, tag, start cloudflared
    holder running                   -> untag, stop own cloudflared
```

Without `--overwrite` the write fails if the key exists. That single atomic write
is the whole election — two instances booting at once cannot both win.

Nothing releases the parameter when a leader dies; termination runs no hook.
Cleanup is lazy: the next instance to boot fails its claim, sees the holder is
gone, and overwrites. Terminate everything, launch one, and the hostnames come
back with no manual step.

Electing on every boot (not just first boot) is what makes taking over from a
*stopped* holder safe. Compose uses `restart: unless-stopped`, so a box that lost
the claim while stopped would otherwise have docker resurrect its connector,
putting two on one tunnel — hence the losing branch stops cloudflared rather than
just skipping it.

`ops.sh promote` / `demote` move the tunnel without relaunching; `demote` also
deletes the parameter so another node can claim it. `ops.sh status` prints
`self=` and `leader=`.

**`Name=zebra-testnet`** is how ops finds the box. The ops workflow with
`instance_id` empty resolves `Name` + `Role=leader` + running and fails unless
that is exactly one instance; set `instance_id` to target any other. Note the IAM
role scopes `ssm:SendCommand` by `ssm:resourceTag/Name` against `instance/*`, so
the `Name` tag is effectively a capability — tagging any instance in the account
`zebra-testnet` grants the ops role shell on it.

Election needs, on the instance profile: `ssm:PutParameter`/`GetParameter`/
`DeleteParameter` on `/zebra/leader`, `ssm:GetParameter`+`kms:Decrypt` on the
tunnel token, `ec2:CreateTags`+`ec2:DeleteTags` on itself scoped to the `Role`
key, and `ec2:DescribeInstances`.

## Genesis

Zebra hard-codes genesis for Regtest only. This network has none and no peers, so
the node parks at `current_height=None` until it is POSTed to `submitblock`.
`ops.sh` does it from the vector in the image:

```sh
hex=$(docker exec zebra-testnet cat /app/zebra-test/src/vectors/block-test-0-000-000.txt | tr -d '[:space:]')
curl -s http://127.0.0.1:18232 -X POST -H 'Content-Type: application/json' \
  -d "{\"jsonrpc\":\"1.0\",\"id\":\"ops\",\"method\":\"submitblock\",\"params\":[\"$hex\"]}"
```

State is ephemeral, so this repeats after every start — `ops.sh` calls it on
`deploy`/`restart`/`start`/`recreate`. Idempotent (already-committed → HTTP 200
`"rejected"`). **Docker restarts do not trigger it**: after a crash the node comes
back empty and stays at height 0 until someone runs `ops.sh genesis`.

## End to end

**1. Build and push** — CI `push-ecr.yaml` on merge, or:

```sh
REPO=496038263219.dkr.ecr.eu-central-1.amazonaws.com/dev-zebra-server
aws ecr get-login-password --region "${AWS_REGION:-eu-central-1}" | docker login --username AWS --password-stdin ${REPO%%/*}
docker build -f testnet-single-node-deploy/dockerfile -t $REPO:latest . && docker push $REPO:latest
```

Must be built from this branch — its `zebra-network/src/config.rs` makes
`funding_streams = []` actually clear the default testnet streams; without it
every block is rejected with `Deferred(-7875000000000)`. 20-40 min cold.

**2. Launch** — Console → Launch Templates → `zebra-testnet` → *Launch instance
from template*. First boot takes 2-3 min and elects.

**3. Genesis + verify**

```sh
aws ssm send-command --region "${AWS_REGION:-eu-central-1}" --instance-ids <id> \
  --document-name AWS-RunShellScript --parameters 'commands=["bash /opt/zebra/ops.sh genesis"]'
curl -s https://rpc.test-zsa.org -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"1.0","id":"1","method":"getblockchaininfo","params":[]}'
```

**4. Drive it** — nothing mines on its own; `zcash_tx_tool` mines via
`getblocktemplate`/`submitblock`. Keep `REGTEST_NETWORK` (`TEST_NETWORK` has no
NU7):

```sh
ZCASH_NODE_ADDRESS=rpc.test-zsa.org ZCASH_NODE_PORT=443 ZCASH_NODE_PROTOCOL=https \
  ./target/release/zcash_tx_tool test-orchard-zsa
```

## ops.sh

`deploy <tag>` · `restart` · `start` · `stop` · `recreate` · `genesis` · `logs` ·
`status` · `promote` · `demote`. Everything that starts the node re-serves
genesis.

Normally driven by the ops workflow. To run an action by hand — the box has no
inbound ports and no SSH key, so it goes over SSM:

```sh
REGION=${AWS_REGION:-eu-central-1}
aws ssm send-command --region "$REGION" --instance-ids <id> \
  --document-name AWS-RunShellScript \
  --parameters 'commands=["bash /opt/zebra/ops.sh <action> [tag]"]'
```

Output comes back separately:

```sh
aws ssm get-command-invocation --region "$REGION" \
  --command-id <command-id> --instance-id <id> \
  --query '[Status,StandardOutputContent,StandardErrorContent]' --output text
```

ECR login expires after 12h and the box only logs in at first boot, so a manual
`deploy` on an older box 401s — `docker login` first.

## Changing a running box

`user_data` runs at first boot only:

```sh
aws ssm start-session --target <id>
sudo vi /opt/zebra/docker-compose.yml
sudo bash /opt/zebra/ops.sh recreate
```

Relaunching from the template is the only way the box provably matches this
directory.
