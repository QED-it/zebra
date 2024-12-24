//! Orchard ZSA test vectors

#![allow(missing_docs)]

use hex::FromHex;
use lazy_static::lazy_static;

/// Represents a serialized ZSA block and its validity status.
pub struct OrchardZSABlock {
    /// Description of the cointent of the block
    pub description: Option<&'static str>,
    /// Serialized byte data of the block.
    pub bytes: Vec<u8>,
    /// Indicates whether the block is valid.
    pub is_valid: bool,
}

fn decode_bytes(hex: &str) -> Vec<u8> {
    <Vec<u8>>::from_hex((hex).trim()).expect("Block bytes are in valid hex representation")
}

lazy_static! {
    pub static ref ORCHARD_ZSA_WORKFLOW_BLOCKS: [Vec<OrchardZSABlock>; 1] = [vec![
        OrchardZSABlock {
            description: Some("Header: (2, 5fe240), Transactions: [ V4, V6 { Transfer: 2, Issue: [ (444bd8, false, [ 1000 ]) ] } ]"),
            bytes: decode_bytes(include_str!("orchard-zsa-0-0.txt")),
            is_valid: true
        },
        OrchardZSABlock {
            description: Some("Header: (3, 4450db), Transactions: [ V4, V6 { Transfer: 2 } ]"),
            bytes: decode_bytes(include_str!("orchard-zsa-0-1.txt")),
            is_valid: true
        },
        OrchardZSABlock {
            description: Some("Header: (4, a1d3c5), Transactions: [ V4, V6 { Transfer: 2, Burn: [ (444bd8, 7) ] }, V6 { Transfer: 2, Burn: [ (444bd8, 2) ] } ]"),
            bytes: decode_bytes(include_str!("orchard-zsa-0-2.txt")),
            is_valid: true
        },
        OrchardZSABlock {
            description: Some("Header: (5, 854bf0), Transactions: [ V4, V6 { Transfer: 2, Issue: [ (444bd8, true, []) ] } ]"),
            bytes: decode_bytes(include_str!("orchard-zsa-0-3.txt")),
            is_valid: true
        },
        OrchardZSABlock {
            description: Some("Header: (6, 84f6a1), Transactions: [ V4, V6 { Transfer: 2, Issue: [ (444bd8, false, [ 2000 ]) ] } ]"),
            bytes: decode_bytes(include_str!("orchard-zsa-0-4.txt")),
            is_valid: false
        },
    ]];
}
