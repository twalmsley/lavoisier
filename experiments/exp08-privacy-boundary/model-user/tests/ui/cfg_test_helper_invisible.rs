// model-lib's #[cfg(test)] helpers are compiled only when model-lib itself is
// the test target. They are not part of the library downstream crates link,
// so this module does not exist from here — even in a test/dev build.
fn main() {
    let _bolt = model_lib::resources::test_helpers::bolt();
}
