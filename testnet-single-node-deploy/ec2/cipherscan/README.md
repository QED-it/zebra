# CipherScan on the ZSATestnet box

[CipherScan](https://github.com/Kenbak/cipherscan) explorer, pointed at the
zebrad already running here rather than its own node.

Upstream's compose bundles `zebrad` and `lightwalletd`; both are dropped.
`api` and `web` join `zebra_default` — the network the zebra stack in
`/opt/zebra` creates — so they reach the node as `zebra-testnet:18232`, and
`cloudflared` reaches them by service name.

## Deploy

```sh
sudo git clone https://github.com/Kenbak/cipherscan /opt/cipherscan
sudo cp docker-compose.yml /opt/cipherscan/docker-compose.qedit.yml
sudo cp env.example /opt/cipherscan/.env.qedit   # then set DB_PASSWORD
cd /opt/cipherscan
sudo docker compose -f docker-compose.qedit.yml --env-file .env.qedit up -d --build
```

The `web` image is a Next.js production build: expect several minutes and ~2 GB
of RAM on this 2 vCPU / 3.8 GB box, competing with zebrad.

## Endpoints

Both go through the same tunnel as the other hostnames, wired in the
`zebra-edge` Cloudflare module:

| URL | origin | |
| --- | --- | --- |
| `cipherscan.test-zsa.org` | `web:3000` | explorer UI |
| `cipherscan-api.test-zsa.org` | `api:3001` | `/health`, `/api/*` (`/` is 404) |

`NEXT_PUBLIC_API_URL` must be a **build arg**, not just runtime env — Next
compiles `NEXT_PUBLIC_*` into the browser bundle. Upstream's `Dockerfile` only
declares `NEXT_PUBLIC_NETWORK`, so `qedit-web.Dockerfile` is derived from it
with the extra `ARG`/`ENV` inserted:

```sh
sed 's|^ENV NEXT_PUBLIC_NETWORK=${NEXT_PUBLIC_NETWORK}|ARG NEXT_PUBLIC_API_URL\nENV NEXT_PUBLIC_API_URL=${NEXT_PUBLIC_API_URL}\n&|' \
  Dockerfile > qedit-web.Dockerfile
```

## Prerequisites installed on the box

`git` and the docker `buildx` plugin — neither ships with the AL2023 bootstrap,
and `docker compose build` fails without buildx.

## Status

`postgres`, `redis` and `api` run. The API reaches the node, connects to
PostgreSQL and Redis, and answers `/health` with `{"status":"ok"}`.

The schema comes from a **separate repo**, `github.com/Kenbak/cipherscan-rust`
— apply `schema/postgres.sql` once (two `role "postgres" does not exist`
errors are expected and harmless; they are GRANTs for a role we do not use):

```sh
git clone --depth 1 https://github.com/Kenbak/cipherscan-rust /opt/cipherscan-rust
docker cp /opt/cipherscan-rust/schema/postgres.sql cipherscan-postgres-1:/tmp/
docker exec cipherscan-postgres-1 psql -U zcash_user -d zcash_explorer -f /tmp/postgres.sql
```

## Not running: the indexer, so the tables stay empty (`select count(*) from blocks` = 0)

Nothing fills those tables yet. The indexer is the Rust binary in that same
repo, and it does **not** read the node over RPC — `ZEBRA_STATE_PATH` is
required and it opens Zebra's RocksDB state directly. Two things block it here:

- **`[state] ephemeral = true`.** There is no persistent state directory, and
  what exists lives inside the `zebra-testnet` container. Running the indexer
  means persistent state plus a shared volume, which changes the design that
  re-serves genesis on every start.
- **State version.** This node writes `state/v27`; the indexer documents
  `state/v28`. The path is configurable but the on-disk format is not, so v27
  may not parse.

## Upstream build bugs worked around

`server/api/Dockerfile` builds with context `./server/api`, but the API requires
siblings outside it (`../../lib/peer-client`, `../signals/api`), and those
siblings need packages from `api/package.json`. `qedit-api.Dockerfile` builds
from `./server`, keeps the whole tree at `/app` with the app at `/app/api`, and
hoists `node_modules` to `/app` so sibling requires resolve. It also creates
`/app/cache` owned by UID 1000, which the API writes to and cannot as shipped.

## Caveats

The explorer targets Mainnet/Testnet. This is a *configured* testnet
(`network_name = "ZSATestnet"`, own magic, NU7 at height 1) with ephemeral
state, so it holds only the genesis block until something mines. Consensus
parameter assumptions are unverified here.
