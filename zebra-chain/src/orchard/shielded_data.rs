//! Orchard shielded data for `V5` `Transaction`s.

use std::{
    cmp::{Eq, PartialEq},
    fmt::{self, Debug},
    io,
};

use byteorder::{ReadBytesExt, WriteBytesExt};
use halo2::pasta::pallas;
use reddsa::{orchard::Binding, orchard::SpendAuth, Signature};

use crate::{
    amount::{Amount, NegativeAllowed},
    block::MAX_BLOCK_BYTES,
    orchard::{tree, Action, Nullifier, ValueCommitment},
    primitives::Halo2Proof,
    serialization::{
        AtLeastOne, SerializationError, TrustedPreallocate, ZcashDeserialize, ZcashSerialize,
    },
};

use super::OrchardFlavorExt;

#[cfg(not(feature = "tx-v6"))]
use super::OrchardVanilla;

#[cfg(feature = "tx-v6")]
use super::OrchardZSA;

/// A bundle of [`Action`] descriptions and signature data.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ShieldedData<V: OrchardFlavorExt> {
    /// The orchard flags for this transaction.
    /// Denoted as `flagsOrchard` in the spec.
    pub flags: Flags,
    /// The net value of Orchard spends minus outputs.
    /// Denoted as `valueBalanceOrchard` in the spec.
    pub value_balance: Amount,
    /// The shared anchor for all `Spend`s in this transaction.
    /// Denoted as `anchorOrchard` in the spec.
    pub shared_anchor: tree::Root,
    /// The aggregated zk-SNARK proof for all the actions in this transaction.
    /// Denoted as `proofsOrchard` in the spec.
    pub proof: Halo2Proof,
    /// The Orchard Actions, in the order they appear in the transaction.
    /// Denoted as `vActionsOrchard` and `vSpendAuthSigsOrchard` in the spec.
    pub actions: AtLeastOne<AuthorizedAction<V>>,
    /// A signature on the transaction `sighash`.
    /// Denoted as `bindingSigOrchard` in the spec.
    pub binding_sig: Signature<Binding>,

    #[cfg(feature = "tx-v6")]
    /// Assets intended for burning
    /// Denoted as `vAssetBurn` in the spec (ZIP 230).
    pub burn: V::BurnType,
}

impl<V: OrchardFlavorExt> fmt::Display for ShieldedData<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fmter = f.debug_struct("orchard::ShieldedData");

        fmter.field("actions", &self.actions.len());
        fmter.field("value_balance", &self.value_balance);
        fmter.field("flags", &self.flags);

        fmter.field("proof_len", &self.proof.zcash_serialized_size());

        fmter.field("shared_anchor", &self.shared_anchor);

        fmter.finish()
    }
}

impl<V: OrchardFlavorExt> ShieldedData<V> {
    /// Iterate over the [`Action`]s for the [`AuthorizedAction`]s in this
    /// transaction, in the order they appear in it.
    pub fn actions(&self) -> impl Iterator<Item = &Action<V>> {
        self.actions.actions()
    }

    /// FIXME: add a doc comment
    pub fn action_commons(&self) -> impl Iterator<Item = ActionCommon> + '_ {
        self.actions.actions().map(|action| action.into())
    }

    /// Collect the [`Nullifier`]s for this transaction.
    pub fn nullifiers(&self) -> impl Iterator<Item = &Nullifier> {
        self.actions().map(|action| &action.nullifier)
    }

    /// Calculate the Action binding verification key.
    ///
    /// Getting the binding signature validating key from the Action description
    /// value commitments and the balancing value implicitly checks that the
    /// balancing value is consistent with the value transferred in the
    /// Action descriptions, but also proves that the signer knew the
    /// randomness used for the Action value commitments, which
    /// prevents replays of Action descriptions that perform an output.
    /// In Orchard, all Action descriptions have a spend authorization signature,
    /// therefore the proof of knowledge of the value commitment randomness
    /// is less important, but stills provides defense in depth, and reduces the
    /// differences between Orchard and Sapling.
    ///
    /// The net value of Orchard spends minus outputs in a transaction
    /// is called the balancing value, measured in zatoshi as a signed integer
    /// cv_balance.
    ///
    /// Consistency of cv_balance with the value commitments in Action
    /// descriptions is enforced by the binding signature.
    ///
    /// Instead of generating a key pair at random, we generate it as a function
    /// of the value commitments in the Action descriptions of the transaction, and
    /// the balancing value.
    ///
    /// <https://zips.z.cash/protocol/protocol.pdf#orchardbalance>
    pub fn binding_verification_key(&self) -> reddsa::VerificationKeyBytes<Binding> {
        let cv: ValueCommitment = self.actions().map(|action| action.cv).sum();
        let cv_balance: ValueCommitment =
            ValueCommitment::new(pallas::Scalar::zero(), self.value_balance);

        let key_bytes: [u8; 32] = (cv - cv_balance).into();
        key_bytes.into()
    }

    /// Provide access to the `value_balance` field of the shielded data.
    ///
    /// Needed to calculate the sapling value balance.
    pub fn value_balance(&self) -> Amount<NegativeAllowed> {
        self.value_balance
    }

    /// Collect the cm_x's for this transaction, if it contains [`Action`]s with
    /// outputs, in the order they appear in the transaction.
    pub fn note_commitments(&self) -> impl Iterator<Item = &pallas::Base> {
        self.actions().map(|action| &action.cm_x)
    }
}

impl<V: OrchardFlavorExt> AtLeastOne<AuthorizedAction<V>> {
    /// Iterate over the [`Action`]s of each [`AuthorizedAction`].
    pub fn actions(&self) -> impl Iterator<Item = &Action<V>> {
        self.iter()
            .map(|authorized_action| &authorized_action.action)
    }
}

/// An authorized action description.
///
/// Every authorized Orchard `Action` must have a corresponding `SpendAuth` signature.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct AuthorizedAction<V: OrchardFlavorExt> {
    /// The action description of this Action.
    pub action: Action<V>,
    /// The spend signature.
    pub spend_auth_sig: Signature<SpendAuth>,
}

impl<V: OrchardFlavorExt> AuthorizedAction<V> {
    /// The size of a single Action
    ///
    /// Actions are 5 * 32 + ENCRYPTED_NOTE_SIZE + 80 bytes so the total size of each Action is 820 bytes.
    /// [7.5 Action Description Encoding and Consensus][ps]
    ///
    /// [ps]: <https://zips.z.cash/protocol/nu5.pdf#actionencodingandconsensus>
    pub const ACTION_SIZE: u64 = 5 * 32 + (V::ENCRYPTED_NOTE_SIZE as u64) + 80;

    /// The size of a single `Signature<SpendAuth>`.
    ///
    /// Each Signature is 64 bytes.
    /// [7.1 Transaction Encoding and Consensus][ps]
    ///
    /// [ps]: <https://zips.z.cash/protocol/nu5.pdf#actionencodingandconsensus>
    pub const SPEND_AUTH_SIG_SIZE: u64 = 64;

    /// The size of a single AuthorizedAction
    ///
    /// Each serialized `Action` has a corresponding `Signature<SpendAuth>`.
    pub const AUTHORIZED_ACTION_SIZE: u64 = Self::ACTION_SIZE + Self::SPEND_AUTH_SIG_SIZE;

    /// The maximum number of actions in the transaction.
    // Since a serialized Vec<AuthorizedAction> uses at least one byte for its length,
    // and the signature is required,
    // a valid max allocation can never exceed this size
    pub const ACTION_MAX_ALLOCATION: u64 = (MAX_BLOCK_BYTES - 1) / Self::AUTHORIZED_ACTION_SIZE;

    // To be but we ensure ACTION_MAX_ALLOCATION is less than 2^16 on compile time
    // (this is a workaround, as static_assertions::const_assert! doesn't work for generics,
    // see TrustedPreallocate for Action<V>)
    const _ACTION_MAX_ALLOCATION_OK: u64 = (1 << 16) - Self::ACTION_MAX_ALLOCATION;
    /* FIXME: remove this
    const ACTION_MAX_ALLOCATION_OK: () = assert!(
        Self::ACTION_MAX_ALLOCATION < 1, //(1 << 16),
        "must be less than 2^16"
    );
    */

    /// Split out the action and the signature for V5 transaction
    /// serialization.
    pub fn into_parts(self) -> (Action<V>, Signature<SpendAuth>) {
        (self.action, self.spend_auth_sig)
    }

    // Combine the action and the spend auth sig from V5 transaction
    /// deserialization.
    pub fn from_parts(
        action: Action<V>,
        spend_auth_sig: Signature<SpendAuth>,
    ) -> AuthorizedAction<V> {
        AuthorizedAction {
            action,
            spend_auth_sig,
        }
    }
}

// TODO: FIXME: Consider moving it to transaction.rs as it's not used here. Or move its usage here from transaction.rs.
/// A struct that contains values of several fields of an `Action` struct.
/// Those fields are used in other parts of the code that call the `orchard_actions()` method of the `Transaction`.
/// The goal of using `ActionCommon` is that it's not a generic, unlike `Action`, so it can be returned from Transaction methods
/// (the fields of `ActionCommon` do not depend on the generic parameter `Version` of `Action`).
pub struct ActionCommon {
    /// A reference to the value commitment to the net value of the input note minus the output note.
    pub cv: super::commitment::ValueCommitment,
    /// A reference to the nullifier of the input note being spent.
    pub nullifier: super::note::Nullifier,
    /// A reference to the randomized validating key for `spendAuthSig`.
    pub rk: reddsa::VerificationKeyBytes<SpendAuth>,
    /// A reference to the x-coordinate of the note commitment for the output note.
    pub cm_x: pallas::Base,
}

impl<V: OrchardFlavorExt> From<&Action<V>> for ActionCommon {
    fn from(action: &Action<V>) -> Self {
        Self {
            cv: action.cv,
            nullifier: action.nullifier,
            rk: action.rk,
            cm_x: action.cm_x,
        }
    }
}

/*
struct AssertBlockSizeLimit<const N: u64>;

impl<const N: u64> AssertBlockSizeLimit<N> {
    const OK: () = assert!(N < (1 << 16), "must be less than 2^16");
}
*/

/// The maximum number of orchard actions in a valid Zcash on-chain transaction V5.
///
/// If a transaction contains more actions than can fit in maximally large block, it might be
/// valid on the network and in the mempool, but it can never be mined into a block. So
/// rejecting these large edge-case transactions can never break consensus.
impl<V: OrchardFlavorExt> TrustedPreallocate for Action<V> {
    fn max_allocation() -> u64 {
        // # Consensus
        //
        // > [NU5 onward] nSpendsSapling, nOutputsSapling, and nActionsOrchard MUST all be less than 2^16.
        //
        // https://zips.z.cash/protocol/protocol.pdf#txnconsensus
        //
        // This acts as nActionsOrchard and is therefore subject to the rule.
        // The maximum value is actually smaller due to the block size limit,
        // but we ensure the 2^16 limit with a static assertion.
        //
        // TODO: FIXME: find a better way to use static check (see https://github.com/nvzqz/static-assertions/issues/40,
        // https://users.rust-lang.org/t/how-do-i-static-assert-a-property-of-a-generic-u32-parameter/76307)?
        // The following expression doesn't work for generics, so a workaround with _ACTION_MAX_ALLOCATION_OK in
        // AuthorizedAction impl is used instead:
        // static_assertions::const_assert!(AuthorizedAction::<V>::ACTION_MAX_ALLOCATION < (1 << 16));
        AuthorizedAction::<V>::ACTION_MAX_ALLOCATION
    }
}

impl TrustedPreallocate for Signature<SpendAuth> {
    fn max_allocation() -> u64 {
        // Each signature must have a corresponding action.
        #[cfg(not(feature = "tx-v6"))]
        let result = Action::<OrchardVanilla>::max_allocation();

        // TODO: FIXME: Check this: V6 is used as it provides the max size of the action.
        // So it's used even for V5 - is this correct?
        #[cfg(feature = "tx-v6")]
        let result = Action::<OrchardZSA>::max_allocation();

        result
    }
}

bitflags! {
    /// Per-Transaction flags for Orchard.
    ///
    /// The spend and output flags are passed to the `Halo2Proof` verifier, which verifies
    /// the relevant note spending and creation consensus rules.
    ///
    /// # Consensus
    ///
    /// > [NU5 onward] In a version 5 transaction, the reserved bits 2..7 of the flagsOrchard
    /// > field MUST be zero.
    ///
    /// <https://zips.z.cash/protocol/protocol.pdf#txnconsensus>
    ///
    /// ([`bitflags`](https://docs.rs/bitflags/1.2.1/bitflags/index.html) restricts its values to the
    /// set of valid flags)
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct Flags: u8 {
        /// Enable spending non-zero valued Orchard notes.
        ///
        /// "the `enableSpendsOrchard` flag, if present, MUST be 0 for coinbase transactions"
        const ENABLE_SPENDS = 0b00000001;
        /// Enable creating new non-zero valued Orchard notes.
        const ENABLE_OUTPUTS = 0b00000010;
    }
}

// We use the `bitflags 2.x` library to implement [`Flags`]. The
// `2.x` version of the library uses a different serialization
// format compared to `1.x`.
// This manual implementation uses the `bitflags_serde_legacy` crate
// to serialize `Flags` as `bitflags 1.x` would.
impl serde::Serialize for Flags {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        bitflags_serde_legacy::serialize(self, "Flags", serializer)
    }
}

// We use the `bitflags 2.x` library to implement [`Flags`]. The
// `2.x` version of the library uses a different deserialization
// format compared to `1.x`.
// This manual implementation uses the `bitflags_serde_legacy` crate
// to deserialize `Flags` as `bitflags 1.x` would.
impl<'de> serde::Deserialize<'de> for Flags {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        bitflags_serde_legacy::deserialize("Flags", deserializer)
    }
}

impl ZcashSerialize for Flags {
    fn zcash_serialize<W: io::Write>(&self, mut writer: W) -> Result<(), io::Error> {
        writer.write_u8(self.bits())?;

        Ok(())
    }
}

impl ZcashDeserialize for Flags {
    fn zcash_deserialize<R: io::Read>(mut reader: R) -> Result<Self, SerializationError> {
        // Consensus rule: "In a version 5 transaction,
        // the reserved bits 2..7 of the flagsOrchard field MUST be zero."
        // https://zips.z.cash/protocol/protocol.pdf#txnencodingandconsensus
        Flags::from_bits(reader.read_u8()?)
            .ok_or_else(|| SerializationError::Parse("invalid reserved orchard flags"))
    }
}
