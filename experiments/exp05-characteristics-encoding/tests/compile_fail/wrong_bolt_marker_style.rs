//! An M6 brass bolt passed where REQ-001 (M8 + Steel + Length15mm) is
//! required, marker style.
use exp05_characteristics_encoding::marker::M6BrassBolt15mm;
use exp05_characteristics_encoding::requirements::fasten_marker;

fn main() {
    let _ = fasten_marker(M6BrassBolt15mm);
}
