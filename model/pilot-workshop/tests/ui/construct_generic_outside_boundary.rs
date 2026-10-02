// R1/F-036: a GENERIC macro-built resource is sealed exactly like a
// non-generic one — the parameterized catalogue bolt cannot be conjured
// outside the defining crate's boundary; its `_seal` field is private
// (E0451), which reads as "you may not create this resource" (F-005).
use pilot_workshop::catalogue::{Bolt, L15, SizeM8, Steel};

fn main() {
    let _conjured: Bolt<SizeM8, Steel, L15> = Bolt {
        _seal: core::marker::PhantomData,
    };
}
