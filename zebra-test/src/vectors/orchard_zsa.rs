//! Orchard ZSA test vectors

#![allow(missing_docs)]

use hex::FromHex;
use lazy_static::lazy_static;

/// Represents a serialized ZSA block and its validity status.
pub struct OrchardZSABlock {
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
            bytes: decode_bytes(include_str!("orchard-zsa-0-0.txt")),
            is_valid: true
        },
        OrchardZSABlock {
            bytes: decode_bytes(include_str!("orchard-zsa-0-1.txt")),
            is_valid: true
        },
        OrchardZSABlock {
            bytes: decode_bytes(include_str!("orchard-zsa-0-2.txt")),
            is_valid: true
        },
        OrchardZSABlock {
            bytes: decode_bytes(include_str!("orchard-zsa-0-3.txt")),
            is_valid: true
        },
        OrchardZSABlock {
            bytes: decode_bytes(include_str!("orchard-zsa-0-4.txt")),
            is_valid: false
        },
    ]];
}
