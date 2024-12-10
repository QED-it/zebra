//! Checks for issuance and burn validity.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use zebra_chain::orchard_zsa::{AssetBase, AssetState, IssuedAssets, IssuedAssetsChange};

use crate::{service::read, SemanticallyVerifiedBlock, ValidateContextError, ZebraDb};

use super::Chain;

pub fn valid_burns_and_issuance(
    finalized_state: &ZebraDb,
    parent_chain: &Arc<Chain>,
    semantically_verified: &SemanticallyVerifiedBlock,
) -> Result<IssuedAssets, ValidateContextError> {
    let mut issued_assets = HashMap::new();
    let mut new_asset_ref_notes = HashMap::new();

    // Burns need to be checked and asset state changes need to be applied per tranaction, in case
    // the asset being burned was also issued in an earlier transaction in the same block.
    for transaction in &semantically_verified.block.transactions {
        let issued_assets_change = IssuedAssetsChange::from_transaction(transaction)
            .ok_or(ValidateContextError::InvalidIssuance)?;

        // Check that no burn item attempts to burn more than the issued supply for an asset, and that there
        // are no duplicate burns of a given asset base within a single transaction.
        let mut burned_assets = HashSet::new();
        for burn in transaction.orchard_burns() {
            let asset_base = burn.asset();
            let asset_state =
                asset_state(finalized_state, parent_chain, &issued_assets, &asset_base)
                    // The asset being burned should have been issued by a previous transaction, and
                    // any assets issued in previous transactions should be present in the issued assets map.
                    .ok_or(ValidateContextError::InvalidBurn)?;

            if !burned_assets.insert(asset_base) || asset_state.total_supply < burn.raw_amount() {
                return Err(ValidateContextError::InvalidBurn);
            } else {
                // Any burned asset bases in the transaction will also be present in the issued assets change,
                // adding a copy of initial asset state to `issued_assets` avoids duplicate disk reads.
                issued_assets.insert(asset_base, asset_state);
            }
        }

        for (asset_base, change) in issued_assets_change.iter() {
            let prev_asset_state =
                asset_state(finalized_state, parent_chain, &issued_assets, &asset_base);

            if prev_asset_state.is_none() {
                let first_note_commitment = transaction
                    .orchard_issue_actions()
                    .flat_map(|action| action.notes())
                    .find_map(|note| {
                        (note.asset() == asset_base).then_some(note.commitment().into())
                    })
                    .expect("tx should have an issue action for new asset");

                new_asset_ref_notes.insert(asset_base, first_note_commitment);
            }

            let updated_asset_state = prev_asset_state
                .unwrap_or_default()
                .apply_change(change)
                .ok_or(ValidateContextError::InvalidIssuance)?;

            issued_assets.insert(asset_base, updated_asset_state);
        }
    }

    Ok((issued_assets, new_asset_ref_notes).into())
}

fn asset_state(
    finalized_state: &ZebraDb,
    parent_chain: &Arc<Chain>,
    issued_assets: &HashMap<AssetBase, AssetState>,
    asset_base: &AssetBase,
) -> Option<AssetState> {
    issued_assets
        .get(asset_base)
        .copied()
        .or_else(|| read::asset_state(Some(parent_chain), finalized_state, asset_base))
}
