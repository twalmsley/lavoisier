// R16/R1: History is sealed — "the past" cannot be conjured. A history is
// created only at the system boundary (`history::boundary::new_history`), one
// per concurrent branch, and its entry list is appended only by the recording
// machinery, so a record can never be fabricated from outside. A literal
// construction is a privacy error (E0451) that reads as "you may not create
// this resource" (F-005).
use model_core::history::History;

fn main() {
    let _conjured = History {
        entries: Vec::new(),
        _seal: (),
    };
}
