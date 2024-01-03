use std::io;

use crate::{
    parameters::TX_V5_VERSION_GROUP_ID,
    transaction::{block, LockTime},
};

use super::hasher::Hasher;

const ZCASH_HEADERS_HASH_PERSONALIZATION: &[u8; 16] = b"ZTxIdHeadersHash";

pub(crate) fn hash_header(
    version: u32,
    consensus_branch_id: u32,
    lock_time: LockTime,
    expiry_height: block::Height,
) -> Result<Hasher, io::Error> {
    Hasher::new(ZCASH_HEADERS_HASH_PERSONALIZATION)
        .add(version)?
        .add(TX_V5_VERSION_GROUP_ID)?
        .add(consensus_branch_id)?
        .add(lock_time)?
        .add(expiry_height.0)
}
