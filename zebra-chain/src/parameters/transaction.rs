//! Transaction consensus and utility parameters.

/// The version group ID for Overwinter transactions.
pub const OVERWINTER_VERSION_GROUP_ID: u32 = 0x03C4_8270;

/// The version group ID for Sapling transactions.
pub const SAPLING_VERSION_GROUP_ID: u32 = 0x892F_2085;

/// The version group ID for version 5 transactions.
///
/// Orchard transactions must use transaction version 5 and this version
/// group ID. Sapling transactions can use v4 or v5 transactions.
pub const TX_V5_VERSION_GROUP_ID: u32 = 0x26A7_270A;

/// The version group ID for version 6 transactions.
/// TODO: update this after it's chosen
// FIXME: The upstream version uses 0xFFFF_FFFF for TX_V6_VERSION_GROUP_ID,
// but it was changed here to 0x7777_7777 for compatibility with the value of
// V6_VERSION_GROUP_ID in the ZSA version of librustzcash/zcash_protocol.
// Update to the proper value once it's fixed in librustzcash.
pub const TX_V6_VERSION_GROUP_ID: u32 = 0x7777_7777;
