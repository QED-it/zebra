//! Tests for canonical Orchard proof sizes.

use crate::orchard::{shielded_data::expected_proof_size, OrchardVanilla};

#[cfg(all(zcash_unstable = "nu7", feature = "tx_v6"))]
use crate::orchard::OrchardZSA;

/// The canonical Orchard proof size for `n` actions is:
///
/// - `2272·n + 2720` bytes for `OrchardVanilla`;
/// - `2272·n + 2848` bytes for `OrchardZSA`.
///
/// These values match the corresponding Orchard circuits' `halo2_proofs`
/// `CircuitCost` results and are consensus-critical, so pin them here.
#[test]
fn expected_proof_size_known_values() {
    assert_eq!(expected_proof_size::<OrchardVanilla>(0), 2720);
    assert_eq!(expected_proof_size::<OrchardVanilla>(1), 4992);
    assert_eq!(expected_proof_size::<OrchardVanilla>(2), 7264);
    assert_eq!(expected_proof_size::<OrchardVanilla>(3), 9536);

    #[cfg(all(zcash_unstable = "nu7", feature = "tx_v6"))]
    {
        assert_eq!(expected_proof_size::<OrchardZSA>(0), 2848);
        assert_eq!(expected_proof_size::<OrchardZSA>(1), 5120);
        assert_eq!(expected_proof_size::<OrchardZSA>(2), 7392);
        assert_eq!(expected_proof_size::<OrchardZSA>(3), 9664);
    }
}
