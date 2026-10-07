// R12/F-015/F-034: the bin's Consumer impl exists only while space remains.
// After its 80-piece capacity is consumed the space parameter is Zero — the
// state a bin reaches at its 81st piece — and feeding it anything more is
// E0277 with model-core's modeller-phrased Consumer message ("it is full").
// (Spelling out 80 consumes in a ui case would pin an 80-deep type in the
// stderr file, F-009; the space-Zero state is the same refusal.)
use cs4_line::resources::boundary::new_swarf_bin;
use cs4_stores::resources::CutSwarf;
use model_core::boundary::send_to;
use model_core::nat::aliases::N0;

fn main() {
    // A bin with no space left: the 81st-piece state of the 80-slot bin.
    let full_bin = new_swarf_bin::<N0>();
    let bin = send_to(full_bin, CutSwarf::<500>::test_fixture());
}
