//! Integration tests, run from the downstream side of the crate boundary.

use model_lib::processes;

/// The user crate can run a process end to end using only the boundary.
#[test]
fn user_crate_runs_process_end_to_end() {
    let (assembly, _spares) = model_user::assemble_one();
    let (_b, _t, _bt) = assembly.disassemble();
}

/// The `test-support` feature (enabled here via dev-dependencies only) lets
/// downstream *tests* build fixtures without the boundary.
#[test]
fn test_support_fixtures_work_in_downstream_tests() {
    let bolt = model_lib::test_support::bolt_fixture();
    let top = model_lib::test_support::plate_fixture();
    let bottom = model_lib::test_support::plate_fixture();
    let assembly = processes::fasten(bolt, top, bottom);
    let (_b, _t, _bt) = assembly.disassemble();
}
