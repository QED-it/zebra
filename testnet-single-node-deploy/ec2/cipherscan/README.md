# CipherScan on the ZSA testnet box

[CipherScan](https://github.com/QED-it/cipherscan) explorer against the zebrad
already running here. Its services live in `../docker-compose.yml` with the node
— one compose for the whole box. Only the build contexts point elsewhere
(`${CIPHERSCAN_DIR}`, an upstream clone), because the images need that tree.

`api` and `web` must keep those service names: the Cloudflare tunnel resolves
origins by service name.

| | |
| --- | --- |
| `web:3000` | `cipherscan.test-zsa.org` — UI |
| `api:3001` | `cipherscan-api.test-zsa.org` — `/health`, `/api/*` |

## Deploy

No SSH; go over SSM (`aws ssm start-session --target <id>`). One-time:

```sh
sudo dnf install -y git docker-buildx-plugin        # not in the AL2023 bootstrap
sudo git clone https://github.com/QED-it/cipherscan /opt/cipherscan
# append env.example to /opt/zebra/.env, set DB_PASSWORD
```

The fork carries `qedit-api.Dockerfile` and `qedit-web.Dockerfile`, so nothing
is copied in from this repo. The copies here are the reviewable source of those
two files.

Then from `/opt/zebra`. **Order matters** — `next build` prerenders pages by
calling the API, so `api` must be up with a complete schema before `web` builds:

```sh
sudo docker compose build api
sudo docker compose up -d postgres api
# schema, below
sudo docker compose build web && sudo docker compose up -d
```

Schema lives in a third repo, and **needs two passes** — one leaves
`transparent_key_exposures` missing, which 500s `/api/rich-list` and fails the
`web` build:

```sh
sudo git clone --depth 1 https://github.com/Kenbak/cipherscan-rust /opt/cipherscan-rust
sudo docker cp /opt/cipherscan-rust/schema/postgres.sql zebra-postgres-1:/tmp/
for i in 1 2; do sudo docker exec zebra-postgres-1 psql -U zcash_user -d zcash_explorer -f /tmp/postgres.sql; done
```

40 `role "postgres" does not exist` errors are expected. Verify with
`select count(*) from pg_tables where schemaname='public'` — should be 35.

`web` is a Next.js production build: several minutes and ~2 GB RAM on this
2 vCPU box, competing with zebrad.

## Why the Dockerfiles exist

Upstream ships none that work here. Each patch fails the build rather than
silently reverting if upstream rewords the line.

- **`web.Dockerfile`** — `lib/api-config.ts` hardcodes
  `POSTGRES_API_URLS['testnet']` to upstream's public API, so the UI served the
  *real* Zcash testnet and ignored this node. Patched to honour
  `NEXT_PUBLIC_API_URL`, which must also be a build ARG since Next inlines
  `NEXT_PUBLIC_*` into the browser bundle.
- **`api.Dockerfile`** — upstream builds with context `./server/api` but the app
  requires siblings outside it. Builds from `./server`, hoists `node_modules`,
  creates the `/app/cache` it writes to, and unpins `listen(PORT, '127.0.0.1')`.

No Redis: the fork stubs the client permanently closed. Every call site already
guarded on `isOpen`, so the list cache falls back to in-process memory, the
WebSocket rate limiter to its in-process limiter, and the broadcast to local
clients — which is all a single API instance ever needed.
- **`indexer.Dockerfile`** — `cipherscan-rust` ships no Dockerfile at all.

## Indexer

Fills the tables the API reads by opening the node's RocksDB in secondary mode.
Behind `profiles: [indexer]`.

Upstream's build parses this chain's v6 transactions as NU6.3 Ironwood and fails
on every block (`expected TX_V6_VERSION_GROUP_ID` — this node writes
`0x7777_7777`, upstream expects `0xD884_B698`). Porting it to ZSA needs, in the
`cipherscan-rust` tree:

1. `zebra-chain` → `{ git = "QED-it/zebra", branch = "zsa1", features = ["tx_v6"] }`
2. QED-it/zebra's `[patch.crates-io]` block, copied verbatim — `[patch]` only
   applies from the root manifest, so it is not inherited
3. QED-it/zebra's `Cargo.lock` as the starting lock — it pins crates crates.io
   has since **yanked** (`core2 0.3.3`, `halo2_gadgets 0.4.0`), which a fresh
   resolve refuses
4. `.cargo/config.toml` with `--cfg zcash_unstable="nu7"` — V6 is gated on both
   that and the feature
5. In `src/indexer/transactions.rs`: drop `.data()` on the Orchard bundle (×3,
   ZSA's `ShieldedData<OrchardZSA>` is the bundle) and replace
   `ironwood_shielded_data` with zero/None (×3, no Ironwood in ZSA v6)

Rust ≥ 1.87. Patch: `cipherscan-rust-zsa-port.patch`. With it applied the
indexer syncs the chain and the explorer shows this node's blocks; ZSA burn and
issuance are still unindexed.

## The node's own service

`zebra-testnet` takes `ZEBRA_CONFIG` and `ZEBRA_STATE_EPHEMERAL`. Unset, it
behaves as before. This box sets both — it runs a bind-mounted
`/etc/zebra/zebrad.toml` (`internal_miner = true`, `ephemeral = false`) with
state in `/var/lib/zebra-state`, which the indexer needs. `ZEBRA_STATE_PATH`
follows from it: `state/v<DB_VERSION>/<network_name lowercased>`.

## Caveats

Explorer targets Mainnet/Testnet; this is a configured testnet with its own
magic and NU7 at height 1, so its consensus assumptions are unverified here. The
chain only grows when the internal miner or `zcash_tx_tool` produces a block.
