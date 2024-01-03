use std::io;

use crate::orchard::{OrchardFlavour, ShieldedData};

use super::hasher::Hasher;

const ZCASH_ORCHARD_HASH_PERSONALIZATION: &[u8; 16] = b"ZTxIdOrchardHash";
const ZCASH_ORCHARD_ACTIONS_COMPACT_HASH_PERSONALIZATION: &[u8; 16] = b"ZTxIdOrcActCHash";
const ZCASH_ORCHARD_ACTIONS_MEMOS_HASH_PERSONALIZATION: &[u8; 16] = b"ZTxIdOrcActMHash";
const ZCASH_ORCHARD_ACTIONS_NONCOMPACT_HASH_PERSONALIZATION: &[u8; 16] = b"ZTxIdOrcActNHash";

fn calculate_action_digests<V: OrchardFlavour>(
    shielded_data: &ShieldedData<V>,
) -> Result<Option<(Hasher, Hasher, Hasher)>, io::Error> {
    if shielded_data.actions.is_empty() {
        Ok(None)
    } else {
        Ok(Some(shielded_data.actions().try_fold(
            (
                Hasher::new(ZCASH_ORCHARD_ACTIONS_COMPACT_HASH_PERSONALIZATION),
                Hasher::new(ZCASH_ORCHARD_ACTIONS_MEMOS_HASH_PERSONALIZATION),
                Hasher::new(ZCASH_ORCHARD_ACTIONS_NONCOMPACT_HASH_PERSONALIZATION),
            ),
            |(compact_hasher, memos_hasher, noncompact_hasher),
             action|
             -> Result<(Hasher, Hasher, Hasher), io::Error> {
                let enc_ciphertext = action.enc_ciphertext.as_ref();

                let enc_ciphertext_compact = &enc_ciphertext[..V::ENCRYPTED_NOTE_COMPACT_SIZE];
                let enc_ciphertext_memos = &enc_ciphertext
                    [V::ENCRYPTED_NOTE_COMPACT_SIZE..V::ENCRYPTED_NOTE_COMPACT_SIZE + 512];
                let enc_ciphertext_noncompact =
                    &enc_ciphertext[V::ENCRYPTED_NOTE_COMPACT_SIZE + 512..];

                Ok((
                    compact_hasher
                        .add(<[u8; 32]>::from(action.nullifier))?
                        .add(<[u8; 32]>::from(action.cm_x))?
                        .add(action.ephemeral_key)?
                        .add(enc_ciphertext_compact)?,
                    memos_hasher.add(enc_ciphertext_memos)?,
                    noncompact_hasher
                        .add(action.cv)?
                        .add(<[u8; 32]>::from(action.rk))?
                        .add(enc_ciphertext_noncompact)?
                        .add(action.out_ciphertext.0)?,
                ))
            },
        )?))
    }
}

pub(crate) fn hash_orchard<V: OrchardFlavour>(
    shielded_data: &Option<ShieldedData<V>>,
) -> Result<Hasher, io::Error> {
    let mut hasher = Hasher::new(ZCASH_ORCHARD_HASH_PERSONALIZATION);

    if let Some(shielded_data) = shielded_data {
        if let Some((actions_compact_digest, actions_memos_digest, actions_noncompact_digest)) =
            calculate_action_digests(shielded_data)?
        {
            hasher = hasher
                .add(actions_compact_digest)?
                .add(actions_memos_digest)?
                .add(actions_noncompact_digest)?
                .add(shielded_data.flags)?
                .add(shielded_data.value_balance())?
                .add(shielded_data.shared_anchor)?;
        }
    }

    Ok(hasher)
}
