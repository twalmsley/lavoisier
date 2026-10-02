//! Failure outputs do not go back to stores (candidate R17): `ToolStores`
//! consumes reserves (`Plate`, `SpareParts`), never scrap. Because the
//! stores have several `Consumer` impls, this misuse takes the trait-bound
//! path and shows model-core's `#[diagnostic::on_unimplemented]` message
//! (F-015) — contrast `scrap_cannot_be_shipped.rs`, where a single-impl
//! consumer produces a plain E0308 instead.

use exp10_failure_modes::model::boundary::new_tool_stores;
use exp10_failure_modes::model::ScrapPlate;
use model_core::boundary::send_to;

fn main() {
    let scrap: ScrapPlate<430> = ScrapPlate::test_fixture();
    let _stores = send_to(new_tool_stores(), scrap);
}
