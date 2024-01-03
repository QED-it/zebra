use std::io;

use crate::{
    transaction,
    transparent::{Input, OutPoint, Output},
};

use super::hasher::Hasher;

const ZCASH_TRANSPARENT_HASH_PERSONALIZATION: &[u8; 16] = b"ZTxIdTranspaHash";

const ZCASH_PREVOUTS_HASH_PERSONALIZATION: &[u8; 16] = b"ZTxIdPrevoutHash";
const ZCASH_SEQUENCE_HASH_PERSONALIZATION: &[u8; 16] = b"ZTxIdSequencHash";
const ZCASH_OUTPUTS_HASH_PERSONALIZATION: &[u8; 16] = b"ZTxIdOutputsHash";

const COINBASE_PREV_OUT: OutPoint = OutPoint {
    hash: transaction::Hash([0; 32]),
    index: 0xffff_ffff,
};

pub(crate) fn hash_transparent(inputs: &[Input], outputs: &[Output]) -> Result<Hasher, io::Error> {
    let (prevouts_digest, sequence_digest) = inputs.iter().cloned().try_fold(
        (
            Hasher::new(ZCASH_PREVOUTS_HASH_PERSONALIZATION),
            Hasher::new(ZCASH_SEQUENCE_HASH_PERSONALIZATION),
        ),
        |(prevouts_hasher, sequence_hasher), input| -> Result<(Hasher, Hasher), io::Error> {
            let (prevout, sequence) = match input {
                Input::PrevOut {
                    outpoint, sequence, ..
                } => (outpoint, sequence),
                Input::Coinbase { sequence, .. } => (COINBASE_PREV_OUT, sequence),
            };

            Ok((
                prevouts_hasher.add(prevout)?,
                sequence_hasher.add(sequence)?,
            ))
        },
    )?;

    let outputs_digest = outputs
        .iter()
        .cloned()
        .try_fold(Hasher::new(ZCASH_OUTPUTS_HASH_PERSONALIZATION), Hasher::add)?;

    Hasher::new(ZCASH_TRANSPARENT_HASH_PERSONALIZATION).add_all_if_any_nonempty(&[
        prevouts_digest,
        sequence_digest,
        outputs_digest,
    ])
}
