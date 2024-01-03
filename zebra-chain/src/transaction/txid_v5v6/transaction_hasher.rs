use std::io;

use crate::transaction::Transaction;

use super::{
    hasher::{HashWriter, Hasher},
    header_hasher, orchard_hasher, sapling_hasher, transparent_hasher,
};

const ZCASH_TX_PERSONALIZATION_PREFIX: &[u8; 12] = b"ZcashTxHash_";

fn calculate_tx_personal(consensus_branch_id: u32) -> Result<[u8; 16], io::Error> {
    let mut tx_personal = [0; 16];
    tx_personal[..12].copy_from_slice(ZCASH_TX_PERSONALIZATION_PREFIX);
    consensus_branch_id.write(&mut tx_personal[12..])?;
    Ok(tx_personal)
}

const TX_V5_ID: u32 = 5;
const OVERWINTER_FLAG: u32 = 1 << 31;

pub(crate) fn hash_txid(tx: &Transaction) -> Result<Option<Hasher>, io::Error> {
    match tx {
        Transaction::V5 {
            network_upgrade,
            lock_time,
            expiry_height,
            inputs,
            outputs,
            sapling_shielded_data,
            orchard_shielded_data,
        } => {
            let consensus_branch_id = u32::from(
                network_upgrade
                    .branch_id()
                    .expect("valid transactions must have a network upgrade with a branch id"),
            );

            let header_digest = header_hasher::hash_header(
                TX_V5_ID | OVERWINTER_FLAG,
                consensus_branch_id,
                *lock_time,
                *expiry_height,
            )?;

            let transparent_digest = transparent_hasher::hash_transparent(inputs, outputs)?;

            let sapling_digest = sapling_hasher::hash_sapling(sapling_shielded_data)?;

            let orchard_digest = orchard_hasher::hash_orchard(orchard_shielded_data)?;

            Ok(Some(
                Hasher::new(&calculate_tx_personal(consensus_branch_id)?)
                    .add(header_digest)?
                    .add(transparent_digest)?
                    .add(sapling_digest)?
                    .add(orchard_digest)?,
            ))
        }

        _ => Ok(None),
    }
}
