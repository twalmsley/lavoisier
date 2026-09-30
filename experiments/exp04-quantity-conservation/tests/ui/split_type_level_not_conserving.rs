// R3/R4: type-level style — splitting 5 g into 3 g + 3 g must not compile.
// Conservation is a trait bound (A: Add<B, Sum = In>), so this fires under
// `cargo check` at the call site.
use exp04_quantity_conservation::type_level_style::{boundary, split, MassG, N3, N5};

fn main() {
    let five = boundary::supply::<N5>();
    let (a, b): (MassG<N3>, MassG<N3>) = split(five);
    let _ = (a, b);
}
