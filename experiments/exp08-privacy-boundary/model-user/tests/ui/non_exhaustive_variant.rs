// The mitigation for the pub-enum hole: a #[non_exhaustive] struct-like
// variant cannot be constructed outside the defining crate.
use model_lib::holes::SealedEnumMitigation;

fn main() {
    let _minted = SealedEnumMitigation::Pristine {};
}
