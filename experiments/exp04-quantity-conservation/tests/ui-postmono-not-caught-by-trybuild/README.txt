These three cases really do fail to compile (`cargo build` rejects them with
E0080), but trybuild runs `cargo check`, which never evaluates the constants
inside monomorphized generic functions. Under trybuild each case reports
"Expected test case to fail to compile, but it succeeded."

They are kept here, OUT of the trybuild glob, as the minimal demonstration of
that blind spot. The same three violations are covered by `compile_fail`
doc-tests in src/lib.rs (rustdoc fully builds doc-tests, so those do catch
the E0080) — that is the workaround this experiment recommends.
