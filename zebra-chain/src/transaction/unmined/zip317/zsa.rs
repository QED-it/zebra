//! The [ZIP-227] ZSA issuance contribution to the [ZIP-317] fee calculation.
//!
//! Note that the 2026-06-26 revision of ZIP-317 removed the ZSA fee contributions; see its
//! [change history](https://zips.z.cash/zip-0317#section-1).
//!
//! [ZIP-227]: https://zips.z.cash/zip-0227.html
//! [ZIP-317]: https://zips.z.cash/zip-0317#fee-calculation

use std::collections::HashSet;

use zcash_primitives::transaction::fees::zip317::CREATION_COST;

use crate::transaction::Transaction;

/// Returns the number of ZIP-317 `logical_actions` contributed by ZIP-227 issuance in
/// `transaction`: `nIssueNotes + CREATION_COST * nReferenceNotes`.
///
/// `nReferenceNotes` is the number of distinct Assets whose Issue Action in this transaction
/// starts with a [reference note].
/// It is used in place of `nAssetCreations`, which the current ZIP-227 defines in terms of the
/// Global Issuance State and which therefore cannot be computed from the transaction alone:
///
/// 1. ZIP-317 requires the conventional fee to be computable from only the public data of the
///    transaction.
/// 2. ZIP-227 chains the issuance state transaction-by-transaction within a block, so a
///    state-dependent count would depend on the transaction's position in the block — which is
///    itself decided by the block template algorithm from the conventional fee.
/// 3. `UnminedTx` caches the conventional fee at construction, so a state-dependent value would
///    go stale across reorgs and over the transaction's mempool lifetime.
/// 4. Reading the Global Issuance State to determine the fact of asset creation means a call
///    from `Verifier::call` in `zebra-consensus` into `zebra-state`, which runs as a separate
///    task. That is theoretically possible — see `mempool_best_chain_next_median_time_past` as
///    an example — but it complicates the code, slows its execution, and is subject to TOCTOU
///    races.
///
/// A reference note is required on the first issuance of an Asset and optional on subsequent
/// issuance, so `nReferenceNotes >= nAssetCreations` and the fee can never be underpaid. It is
/// higher only if an issuer voluntarily places a reference note first when re-issuing an Asset,
/// which has no practical benefit and only raises their own fee. Counting distinct Assets rather
/// than Issue Actions keeps that overcount as small as the transaction data allows.
///
/// [reference note]: https://zips.z.cash/zip-0227.html#reference-notes
//
// TODO: `nReferenceNotes` replaces `nAssetCreations` from ZIP-227. Either amend ZIP-227 to define
// the contribution in these terms, or switch back to `nAssetCreations` here.
pub fn logical_actions(transaction: &Transaction) -> usize {
    let Some(issue_data) = transaction.orchard_zsa_issue_data() else {
        return 0;
    };
    let issue_bundle = issue_data.inner();

    let n_issue_notes = issue_bundle.get_all_notes().len();

    let n_reference_notes = issue_bundle
        .actions()
        .iter()
        .filter_map(|action| action.get_reference_note())
        // An Asset is created at most once, so count each Asset once however many Issue
        // Actions in this transaction carry a reference note for it.
        .map(|note| note.asset().to_bytes())
        .collect::<HashSet<_>>()
        .len();

    n_issue_notes + (CREATION_COST * n_reference_notes)
}
