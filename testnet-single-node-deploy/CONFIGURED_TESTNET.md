# ZSA1_1 — Configured Testnet Setup

Zebra as a private testnet (`network = "Testnet"` + `[network.testnet_parameters]`, not `"Regtest"`) with NU7/ZSA at height 1, no peers, no PoW. Config: `testnet-single-node-deploy/testnet-config.toml`.

## Run

```bash
cargo build --release --package zebrad --bin zebrad
./target/release/zebrad -c testnet-single-node-deploy/testnet-config.toml start
```

Zebra starts with an empty state (no hard-coded Testnet genesis, unlike Regtest). Inject it once via `submitblock`:

```bash
GENESIS_HEX=$(tr -d '[:space:]' < zebra-test/src/vectors/block-test-0-000-000.txt)
curl -s http://localhost:18232 -X POST -H 'Content-Type: application/json' \
  -d "{\"jsonrpc\":\"1.0\",\"id\":\"bootstrap\",\"method\":\"submitblock\",\"params\":[\"$GENESIS_HEX\"]}"
# repeat after every restart — state is ephemeral
```

## zcash_tx_tool

Keep `REGTEST_NETWORK` (not `TEST_NETWORK`, which has no NU7). No code changes needed.

```bash
mkdir /tmp/qed-wallet && cd /tmp/qed-wallet
ZCASH_NODE_ADDRESS=127.0.0.1 ZCASH_NODE_PORT=18232 ZCASH_NODE_PROTOCOL=http \
  /path/to/zcash_tx_tool/target/release/zcash_tx_tool test-orchard-zsa
```
