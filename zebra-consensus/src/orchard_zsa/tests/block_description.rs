use orchard::note::AssetBase;

use zebra_chain::{
    block::Block,
    orchard::{OrchardZSA, ShieldedData},
    orchard_zsa::{BurnItem, IssueData},
    transaction::Transaction,
};

use hex::encode as encode_hex;

fn format_asset(asset: &AssetBase) -> String {
    encode_hex(asset.to_bytes()).chars().take(6).collect()
}

fn format_array(strings: impl IntoIterator<Item = String>) -> String {
    let text = strings.into_iter().collect::<Vec<_>>().join(", ");

    if text.is_empty() {
        "[]".to_string()
    } else {
        format!("[ {text} ]")
    }
}

fn format_transfer(n_actions: usize) -> Option<String> {
    (n_actions > 0).then(|| format!("Transfer: {}", n_actions))
}

fn format_burn(burn_items: &[BurnItem]) -> Option<String> {
    (!burn_items.is_empty()).then(|| {
        let burn = burn_items
            .iter()
            .map(|b| format!("({}, {})", format_asset(&b.asset()), b.raw_amount()));
        format!("Burn: {}", format_array(burn))
    })
}

fn format_issue(issue_data: &IssueData) -> Option<String> {
    let ik = issue_data.inner().ik();

    let issue = issue_data.actions().map(|action| {
        let asset = format_asset(&AssetBase::derive(ik, action.asset_desc()));
        let is_finalized = action.is_finalized();
        let notes = action
            .notes()
            .iter()
            .map(|note| note.value().inner().to_string());
        format!("({}, {}, {})", asset, is_finalized, format_array(notes))
    });

    Some(format!("Issue: {}", format_array(issue)))
}

fn format_tx_v6(
    shielded_data: &Option<ShieldedData<OrchardZSA>>,
    issue_data: &Option<IssueData>,
) -> String {
    let transfer = shielded_data
        .as_ref()
        .and_then(|shielded_data| format_transfer(shielded_data.actions.len()));

    let burn = shielded_data
        .as_ref()
        .and_then(|shielded_data| format_burn(shielded_data.burn.as_ref()));

    let issue = issue_data
        .as_ref()
        .and_then(|issue_data| format_issue(issue_data));

    format!(
        "V6 {{ {} }}",
        [transfer, burn, issue]
            .into_iter()
            .filter_map(|part| part)
            .collect::<Vec<String>>()
            .join(", ")
    )
}

pub(super) fn build_block_description(block: &Block) -> String {
    let height = block
        .coinbase_height()
        .expect("block has coinbase_height")
        .next()
        .expect("block has next coinbase_height")
        .0;

    let hash = &block.hash().to_string()[..6];

    let transactions = format_array(block.transactions.iter().map(|tx| match tx.as_ref() {
        Transaction::V1 { .. } => "V1".to_string(),
        Transaction::V2 { .. } => "V2".to_string(),
        Transaction::V3 { .. } => "V3".to_string(),
        Transaction::V4 { .. } => "V4".to_string(),
        Transaction::V5 { .. } => "V5".to_string(),
        Transaction::V6 {
            orchard_shielded_data,
            orchard_zsa_issue_data,
            ..
        } => format_tx_v6(orchard_shielded_data, orchard_zsa_issue_data),
    }));

    format!(
        "Header: ({}, {}), Transactions: {}",
        height, hash, transactions
    )
}
