//! Regtest and Testnet genesis blocks

use std::sync::Arc;

use hex::FromHex;

use crate::{block::Block, serialization::ZcashDeserializeInto};

/// Genesis block for Regtest, copied from zcashd via `getblock 0 0` RPC method
pub fn regtest_genesis_block() -> Arc<Block> {
    let regtest_genesis_block_bytes =
        <Vec<u8>>::from_hex(include_str!("genesis/block-regtest-0-000-000.txt").trim())
            .expect("Block bytes are in valid hex representation");

    regtest_genesis_block_bytes
        .zcash_deserialize_into()
        .map(Arc::new)
        .expect("hard-coded Regtest genesis block data must deserialize successfully")
}

/// Genesis block for Testnet (and configured testnets that share the Testnet genesis hash),
/// copied from zcashd via `getblock 0 0 -testnet` RPC method
pub fn testnet_genesis_block() -> Arc<Block> {
    let testnet_genesis_block_bytes =
        <Vec<u8>>::from_hex(include_str!("genesis/block-testnet-0-000-000.txt").trim())
            .expect("Block bytes are in valid hex representation");

    testnet_genesis_block_bytes
        .zcash_deserialize_into()
        .map(Arc::new)
        .expect("hard-coded Testnet genesis block data must deserialize successfully")
}
