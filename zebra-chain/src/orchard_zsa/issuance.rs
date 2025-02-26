//! OrchardZSA issuance related functionality.

use std::{fmt::Debug, io};

use halo2::pasta::pallas;

// For pallas::Base::from_repr only
use group::ff::PrimeField;

use zcash_primitives::transaction::components::issuance::{read_v6_bundle, write_v6_bundle};

use orchard::{
    issuance::{IssueAction, IssueBundle, Signed},
    note::ExtractedNoteCommitment,
    Note,
};

use crate::{
    block::MAX_BLOCK_BYTES,
    serialization::{SerializationError, TrustedPreallocate, ZcashDeserialize, ZcashSerialize},
};

use super::burn::ASSET_BASE_SIZE;

/// Wrapper for `IssueBundle` used in the context of Transaction V6. This allows the implementation of
/// a Serde serializer for unit tests within this crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssueData(IssueBundle<Signed>);

// Sizes of the types, in bytes
// FIXME: import from orchard
const ADDRESS_SIZE: u64 = 43;
const NULLIFIER_SIZE: u64 = 32;
const NOTE_VALUE_SIZE: u64 = 4;
const RANDOM_SEED_SIZE: u64 = 32;
// FIXME: is this a correct way to calculate (simple sum of sizes of components)?
const NOTE_SIZE: u64 =
    ADDRESS_SIZE + NOTE_VALUE_SIZE + ASSET_BASE_SIZE + NULLIFIER_SIZE + RANDOM_SEED_SIZE;

impl TrustedPreallocate for Note {
    fn max_allocation() -> u64 {
        (MAX_BLOCK_BYTES - 1) / NOTE_SIZE
    }
}

impl TrustedPreallocate for IssueAction {
    fn max_allocation() -> u64 {
        (MAX_BLOCK_BYTES - 1) / 3
    }
}

impl ZcashSerialize for Option<IssueData> {
    fn zcash_serialize<W: io::Write>(&self, writer: W) -> Result<(), io::Error> {
        write_v6_bundle(self.as_ref().map(|issue_data| &issue_data.0), writer)
    }
}

// FIXME: We can't split IssueData out of Option<IssueData> deserialization,
// because the counts are read along with the arrays.
impl ZcashDeserialize for Option<IssueData> {
    fn zcash_deserialize<R: io::Read>(reader: R) -> Result<Self, SerializationError> {
        Ok(read_v6_bundle(reader)?.map(IssueData))
    }
}

#[cfg(any(test, feature = "proptest-impl"))]
impl serde::Serialize for IssueData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // TODO: FIXME: implement Serde serialization here
        "(IssueData)".serialize(serializer)
    }
}
