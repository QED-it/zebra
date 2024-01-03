use std::io;

use crate::transaction::{self, Transaction};

mod hasher;
mod header_hasher;
mod orchard_hasher;
mod sapling_hasher;
mod transaction_hasher;
mod transparent_hasher;

use transaction_hasher::hash_txid;

pub fn calculate_txid(tx: &Transaction) -> Result<Option<transaction::Hash>, io::Error> {
    match hash_txid(tx)? {
        Some(hasher) => Ok(Some(transaction::Hash::from(
            <[u8; 32]>::try_from(hasher.finalize().as_bytes())
                .expect("Blake2bHash must be convertable to [u8; 32]"),
        ))),
        None => Ok(None),
    }
}
