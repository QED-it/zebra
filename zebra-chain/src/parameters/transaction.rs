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

#[cfg(feature = "tx-v6")]
/// The version group ID for version 6 transactions.
///
/// OrchardZSA transactions must use transaction version 6 and this version
/// group ID.
// FIXME: apparently another value needs to be defined here
// (for now it's simply TX_V5_VERSION_GROUP_ID increased by 1)
pub const TX_V6_VERSION_GROUP_ID: u32 = 0x26A7_270B;
