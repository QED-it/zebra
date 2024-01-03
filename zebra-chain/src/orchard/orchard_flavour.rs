//! This module defines traits and structures for supporting the Orchard Shielded Protocol
//! for `V5` and `V6` versions of the transaction.
use std::{fmt::Debug, io};

use serde::{de::DeserializeOwned, Serialize};

#[cfg(any(test, feature = "proptest-impl"))]
use proptest_derive::Arbitrary;

use crate::serialization::{SerializationError, ZcashDeserialize, ZcashSerialize};

use super::note;

#[cfg(feature = "tx-v6")]
use crate::orchard_zsa::burn::BurnItem;

/// The size of the encrypted note for the Orchard ShieldedData of `V5` transactions.
pub const ENCRYPTED_NOTE_SIZE_V5: usize = 580;

/// The size of the encrypted note for the Orchard ShieldedData of `V6` transactions.
#[cfg(feature = "tx-v6")]
pub const ENCRYPTED_NOTE_SIZE_V6: usize = orchard_zsa::note_encryption_v3::ENC_CIPHERTEXT_SIZE_V3;

/// A trait representing compile-time settings of Orchard Shielded Protocol used in
/// the transactions `V5` and `V6`.
pub trait OrchardFlavour: Clone + Debug {
    /// The size of the encrypted note for this protocol version.
    const ENCRYPTED_NOTE_SIZE: usize;

    /// The size of the part of the encrypted note included in the compact format of Orchard Action.
    /// For detailed information, refer to ZIP 244 ("T.4a: orchard_actions_compact_digest" section)
    /// and ZIP 0307 ("Output Compression" section).
    /// Here it is utilized for the calculation of txid.
    #[cfg(feature = "txid-v5v6")]
    const ENCRYPTED_NOTE_COMPACT_SIZE: usize;

    /// A type representing an encrypted note for this protocol version.
    type EncryptedNote: Clone
        + Debug
        + PartialEq
        + Eq
        + DeserializeOwned
        + Serialize
        + ZcashDeserialize
        + ZcashSerialize
        + AsRef<[u8]>;

    /// A type representing a burn field for this protocol version.
    type BurnType: Clone + Debug + Default + ZcashDeserialize + ZcashSerialize;
}

/// A structure representing a tag for Orchard protocol variant used for the transaction version `V5`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(any(test, feature = "proptest-impl"), derive(Arbitrary))]
pub struct Orchard;

/// A structure representing a tag for Orchard protocol variant used for the transaction version `V6`
/// (which ZSA features support).
#[cfg(feature = "tx-v6")]
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(any(test, feature = "proptest-impl"), derive(Arbitrary))]
pub struct OrchardZSA;

/// A special marker type indicating the absence of a burn field in Orchard ShieldedData for `V5` transactions.
/// Useful for unifying ShieldedData serialization and deserialization implementations across various
/// Orchard protocol variants (i.e. various transaction versions).
#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NoBurn;

impl ZcashSerialize for NoBurn {
    fn zcash_serialize<W: io::Write>(&self, mut _writer: W) -> Result<(), io::Error> {
        Ok(())
    }
}

impl ZcashDeserialize for NoBurn {
    fn zcash_deserialize<R: io::Read>(mut _reader: R) -> Result<Self, SerializationError> {
        Ok(Self)
    }
}

// The following implementations of the `AsRef<[u8]>` trait are required for the direct
// implementation of transaction ID calculation for Orchard ShieldedData of transactions
// `V5` and `V6`.
impl AsRef<[u8]> for note::EncryptedNote<ENCRYPTED_NOTE_SIZE_V5> {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

#[cfg(feature = "tx-v6")]
impl AsRef<[u8]> for note::EncryptedNote<ENCRYPTED_NOTE_SIZE_V6> {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

impl OrchardFlavour for Orchard {
    const ENCRYPTED_NOTE_SIZE: usize = ENCRYPTED_NOTE_SIZE_V5;
    #[cfg(feature = "txid-v5v6")]
    const ENCRYPTED_NOTE_COMPACT_SIZE: usize = 52;
    type EncryptedNote = note::EncryptedNote<ENCRYPTED_NOTE_SIZE_V5>;
    type BurnType = NoBurn;
}

#[cfg(feature = "tx-v6")]
impl OrchardFlavour for OrchardZSA {
    const ENCRYPTED_NOTE_SIZE: usize = ENCRYPTED_NOTE_SIZE_V6;
    #[cfg(feature = "txid-v5v6")]
    const ENCRYPTED_NOTE_COMPACT_SIZE: usize = 84;
    type EncryptedNote = note::EncryptedNote<ENCRYPTED_NOTE_SIZE_V6>;
    type BurnType = Vec<BurnItem>;
}
