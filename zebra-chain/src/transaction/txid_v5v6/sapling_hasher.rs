use std::io;

use crate::sapling::{SharedAnchor, ShieldedData};

use super::hasher::Hasher;

const ZCASH_SAPLING_HASH_PERSONALIZATION: &[u8; 16] = b"ZTxIdSaplingHash";

const ZCASH_SAPLING_SPENDS_HASH_PERSONALIZATION: &[u8; 16] = b"ZTxIdSSpendsHash";
const ZCASH_SAPLING_SPENDS_COMPACT_HASH_PERSONALIZATION: &[u8; 16] = b"ZTxIdSSpendCHash";
const ZCASH_SAPLING_SPENDS_NONCOMPACT_HASH_PERSONALIZATION: &[u8; 16] = b"ZTxIdSSpendNHash";

const ZCASH_SAPLING_OUTPUTS_HASH_PERSONALIZATION: &[u8; 16] = b"ZTxIdSOutputHash";
const ZCASH_SAPLING_OUTPUTS_COMPACT_HASH_PERSONALIZATION: &[u8; 16] = b"ZTxIdSOutC__Hash";
const ZCASH_SAPLING_OUTPUTS_MEMOS_HASH_PERSONALIZATION: &[u8; 16] = b"ZTxIdSOutM__Hash";
const ZCASH_SAPLING_OUTPUTS_NONCOMPACT_HASH_PERSONALIZATION: &[u8; 16] = b"ZTxIdSOutN__Hash";

fn hash_sapling_spends(shielded_data: &ShieldedData<SharedAnchor>) -> Result<Hasher, io::Error> {
    let anchor_bytes = shielded_data.shared_anchor().map(<[u8; 32]>::from);

    let (spends_compact_digest, spends_noncompact_digest) = shielded_data.spends().try_fold(
        (
            Hasher::new(ZCASH_SAPLING_SPENDS_COMPACT_HASH_PERSONALIZATION),
            Hasher::new(ZCASH_SAPLING_SPENDS_NONCOMPACT_HASH_PERSONALIZATION),
        ),
        |(compact_hasher, noncompact_hasher), spend| -> Result<(Hasher, Hasher), io::Error> {
            Ok((
                compact_hasher.add(<[u8; 32]>::from(spend.nullifier))?,
                noncompact_hasher
                    .add(spend.cv)?
                    // shared_anchor must present if shielded_data has at least one spends,
                    // so we can safely use unwrap here
                    .add(anchor_bytes.unwrap())?
                    .add(<[u8; 32]>::from(spend.rk.clone()))?,
            ))
        },
    )?;

    Hasher::new(ZCASH_SAPLING_SPENDS_HASH_PERSONALIZATION)
        .add_all_if_any_nonempty(&[spends_compact_digest, spends_noncompact_digest])
}

fn hash_sapling_outputs(shielded_data: &ShieldedData<SharedAnchor>) -> Result<Hasher, io::Error> {
    let (outputs_compact_digest, outputs_memos_digest, outputs_noncompact_digest) =
        shielded_data.outputs().try_fold(
            (
                Hasher::new(ZCASH_SAPLING_OUTPUTS_COMPACT_HASH_PERSONALIZATION),
                Hasher::new(ZCASH_SAPLING_OUTPUTS_MEMOS_HASH_PERSONALIZATION),
                Hasher::new(ZCASH_SAPLING_OUTPUTS_NONCOMPACT_HASH_PERSONALIZATION),
            ),
            |(compact_hasher, memos_hasher, noncompoact_hasher),
             output|
             -> Result<(Hasher, Hasher, Hasher), io::Error> {
                let enc_ciphertext = output.enc_ciphertext.0;

                let enc_ciphertext_compact = &enc_ciphertext[..52];
                let enc_ciphertext_memos = &enc_ciphertext[52..564];
                let enc_ciphertext_noncompact = &enc_ciphertext[564..];

                Ok((
                    compact_hasher
                        .add(output.cm_u.to_bytes())?
                        .add(output.ephemeral_key)?
                        .add(enc_ciphertext_compact)?,
                    memos_hasher.add(enc_ciphertext_memos)?,
                    noncompoact_hasher
                        .add(output.cv)?
                        .add(enc_ciphertext_noncompact)?
                        .add(output.out_ciphertext.0)?,
                ))
            },
        )?;

    Hasher::new(ZCASH_SAPLING_OUTPUTS_HASH_PERSONALIZATION).add_all_if_any_nonempty(&[
        outputs_compact_digest,
        outputs_memos_digest,
        outputs_noncompact_digest,
    ])
}

pub(crate) fn hash_sapling(
    shielded_data: &Option<ShieldedData<SharedAnchor>>,
) -> Result<Hasher, io::Error> {
    let mut hasher = Hasher::new(ZCASH_SAPLING_HASH_PERSONALIZATION);

    if let Some(shielded_data) = shielded_data {
        let sapling_spends_digest = hash_sapling_spends(shielded_data)?;
        let sapling_outputs_digest = hash_sapling_outputs(shielded_data)?;

        if !(sapling_spends_digest.is_empty() && sapling_outputs_digest.is_empty()) {
            hasher = hasher
                .add(sapling_spends_digest)?
                .add(sapling_outputs_digest)?
                .add(shielded_data.value_balance())?
        }
    }

    Ok(hasher)
}
