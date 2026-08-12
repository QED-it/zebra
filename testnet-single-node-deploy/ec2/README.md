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

## Leader election

A Cloudflare Tunnel has one token, and every `cloudflared` holding it registers
as another connector — Cloudflare then round-robins the public hostnames across
them. That is meant for replicas. These nodes are not replicas: state is
ephemeral, so each has its own chain, and two connectors means one hostname
answering from two different chains. Hence exactly one instance may run
`cloudflared`.

**`Role=leader` marks that instance.** Only the leader carries the tag; every
other instance has no `Role` tag.

Nothing on the box elects, and nothing on the box writes the tag. `leader.sh
apply` reads it and makes the box match:

```
Role=leader present -> ensure cloudflared up
absent              -> ensure cloudflared down
```

That runs at boot (`zebra-leader.service`) and on demand (`ops.sh apply`). There
is no shared lock, no claim, and no state two boxes can disagree about — the tag
is written in one place and every box is a follower of it.

The ops workflow owns the verbs:

| | |
| --- | --- |
| `promote B` | untag the current leader, tag B, `apply` to both |
| `demote` | untag, `apply` |
| leader terminated | nothing reclaims automatically — `promote` the replacement |
| `up` fails | `apply` exits non-zero; run it again |

Two things keep a stale connector from coming back. `cloudflared` sits in the
`tunnel` compose profile, so a bare `docker compose up -d` never starts one —
only `leader.sh` does. And `restart: unless-stopped` revives the leader's
connector after a reboot, which `apply` at boot undoes on a box that lost the tag
while it was stopped.

Instance profile needs only `ec2:DescribeTags` and `ssm:GetParameter`+
`kms:Decrypt` on the tunnel token — no write permissions at all. Enabling
instance metadata tags on the launch template would drop `ec2:DescribeTags` too,
making `is_leader` a plain IMDS read.

**`Name=zebra-testnet`** is how ops finds the box. The IAM role scopes
`ssm:SendCommand` by `ssm:resourceTag/Name` against `instance/*`, so that tag is
effectively a capability — tagging any instance in the account `zebra-testnet`
grants the ops role shell on it.

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
aws ecr get-login-password --region eu-central-1 | docker login --username AWS --password-stdin ${REPO%%/*}
docker build -f testnet-single-node-deploy/dockerfile -t $REPO:latest . && docker push $REPO:latest
```

Must be built from this branch — its `zebra-network/src/config.rs` makes
`funding_streams = []` actually clear the default testnet streams; without it
every block is rejected with `Deferred(-7875000000000)`. 20-40 min cold.

**2. Launch** — Console → Launch Templates → `zebra-testnet` → *Launch instance
from template*. First boot takes 2-3 min and elects.

**3. Genesis + verify**

```sh
aws ssm send-command --region eu-central-1 --instance-ids <id> \
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
`status` · `apply`. Everything that starts the node re-serves genesis.
`promote`/`demote` are workflow actions, not box actions — see above.

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
