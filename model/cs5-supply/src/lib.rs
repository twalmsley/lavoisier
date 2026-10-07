//! # cs5-supply — case study CS-5, the supply subsystem
//!
//! The fifth spec→model round trip, and the **boundary-refinement capstone**:
//! CS-5 is implemented downstream of `model-core` from the agreed
//! natural-language specification `case-studies/cs5-two-site/SPEC.md` (v0.2,
//! agreed) as **three team-shaped crates** (SPEC §3/§8 review decision 1).
//! This crate is **supply**: it owns the euro-cent currency dimension (R19 —
//! a new currency is one `impl Unit` line plus one sealed cash type), the
//! bureau and its `exchange` process (R19's last unexercised clause:
//! boundary-only, integer-exact value-equivalence), the **EU vendor refined
//! from an unbounded placeholder into a finite 3-component stock** (the R12
//! evolution path, finally walked — see [`resources::Vendor`] for the
//! before/after), the boxed precision component with its packaging, and the
//! purchase order the vendor sells against. The sibling crates
//! `cs5-logistics` (the contracted courier) and `cs5-works` (the UK
//! workshop) depend on it: the crate edge carries the currency and component
//! seals (EXP-08, F-005/F-006), so neither downstream crate can mint a euro
//! cent or a component.
//!
//! ## Module map
//!
//! | Module | Contents |
//! |---|---|
//! | [`units`] | The euro-cent unit type (R19: one `impl Unit` line is the whole quantity-level cost of a currency) |
//! | [`characteristics`] | The sealed characteristics behind REQ-023/REQ-024 and the works' REQ-025 ([`characteristics::PricedInEuroCents`], [`characteristics::ExchangeDesk`], [`characteristics::GoodsInInspected`]) |
//! | [`requirements`] | REQ-023 and REQ-024 as R10 requirement traits, each with a REQ-phrased `on_unimplemented` (R10 rule 8, F-044). The workspace trace namespace is global (F-053): the pilot and CS-1..4 own REQ-001..022, so CS-5 starts at REQ-023 exactly as SPEC §2 allocates |
//! | [`resources`] | The sealed resource family (R1): GBP and euro-cent cash (two dimensions that never meet, R19/F-051), the bureau, the refined vendor and its 3-deep stock, the boxed/inspected component states, the packaging and its disposal sink, the purchase order — with `boundary` and `processes` child modules (F-006, F-031) |
//!
//! ## The two currencies (R19, F-051, F-052)
//!
//! GBP cash ([`resources::Money`], pence) and euro-cent cash
//! ([`resources::Euros`]) are **separate dimensions**: two sealed types with
//! no conversion function anywhere in the model — a cross-currency payment
//! is an E0308, like adding grams to millimetres. The only bridge is the
//! bureau's [`resources::processes::exchange`] (REQ-024), a boundary process
//! const-asserting `OUT × 100 == IN × 117` — exact multiplication, no
//! division, so **silent rounding is unrepresentable** (F-052): an amount
//! with no exact exchange at the rate has no `OUT` that compiles (the spec's
//! amounts, 2000 p → 2340 ec, are chosen remainder-free; SPEC §7).
//!
//! ## The refinement showcase (R12, SPEC §4)
//!
//! Every earlier vendor in this workspace was an unbounded `Next = Self`
//! placeholder. CS-5's [`resources::Vendor`] is the **refined** form: a
//! modelled organisation whose stock is a finite 3-deep type-level list — a
//! fourth purchase is a compile error, and exhaustion produces an
//! empty-stock state ([`resources::ExhaustedVendor`]) the vendor must
//! account for. The [`resources::Bureau`] and the works' customer
//! deliberately **stay** placeholders, so both shapes sit side by side.
//!
//! ## Conservation regime (R1)
//!
//! This crate carries the full obligations listed in `model-core`'s crate
//! docs: `recursion_limit = "2048"`, `forbid(unsafe_code)`, the
//! must-use/underscore lints, tripwire `Drop`s on every consumable (kernel
//! macros), and CI clippy under `-D warnings` with the workspace
//! `clippy.toml` ban list. Never write `let _ = <resource>`, and ignore
//! compiler fix-its suggesting borrows, clones, drops or `.expect(…)`
//! (F-007).

#![recursion_limit = "2048"]
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]
#![warn(missing_docs)]

pub mod characteristics;
pub mod requirements;
pub mod resources;
pub mod units;
