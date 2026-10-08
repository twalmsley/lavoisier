//! The batch flow (SPEC.md §6): P1 → P2 → P3 → P5 with P4 anywhere before
//! P6, then the two sequential bakes — composed from the public model API
//! only (boundary constructors, the processes, the `Consumer` sinks) — no
//! `mint`/`defuse` — so it composes exactly the way a downstream flow would.
//!
//! Hand-written past the scaffold (the scaffold's SPEC-HOLE U-14): the
//! generated flow skeletons injected success tokens only; the real flow
//! returns the `#[must_use]` four-combination outcome grouping
//! [`BatchOutcome`] (R17/F-050) and is driven through all four combinations
//! by the integration tests in `tests/flows.rs`.
//!
//! Four structural facts about this flow (R17):
//!
//! 1. **Shared spends are drawn before the branch** (F-048): each bake's
//!    300 000 ms is an adjacent `draw_time` in the flow, recorded into the
//!    `History` (R16) before the fallible step runs — so the baker returns
//!    at the same budget in every combination and a scorch costs time too.
//! 2. **The four combinations are a four-way sum** (F-046/F-050): both
//!    baked, first scorched, second scorched and both scorched end with
//!    different loaf counts at the household and the compost stream, so the
//!    flow returns the `#[must_use]` outcome enum [`BatchOutcome`], one
//!    variant per combination. Everything else ends **identically** — the
//!    spec's "all four draw the same time and energy" is stated in the types
//!    by the single [`KitchenAtRest`] grouping every combination returns.
//! 3. **Rework is inexpressible** (F-050): a scorched loaf is final (the
//!    dough is consumed, SPEC.md §5 P6), exactly two tokens are provisioned
//!    — one per bake — and a third bake has no shaped loaf, no greased tin
//!    and no token to consume.
//! 4. **The two bakes are forced sequential by the single [`Oven`]** (R2):
//!    the second bake can only be written with the oven the first bake
//!    returns — starting both at once is the contention error at the exact
//!    line (R9; the trybuild case `tests/ui/second_bake_before_oven_returned.rs`).
//!
//! **Ordering freedom is deliberately small** (SPEC.md §6: one actor): P4
//! may run anywhere before P6. [`bake_batch`] greases the tins after the
//! prove (order a: P1 P2 P3 P4 P5 P6 P6); [`bake_batch_tins_first`] greases
//! them before the mix (order b: P4 P1 P2 P3 P5 P6 P6). Both compile and
//! both end in the identical per-combination states — the same
//! [`BatchOutcome`] variants and the same [`KitchenAtRest`] type-check,
//! which is the type system proving the orderings equivalent (R9).
//!
//! REQ-030 and REQ-031 ride the flow's consumer parameters (style A,
//! F-019/F-044): the household parameter is bounded by `Req030…` and
//! consumes only `baked` loaves, the compost parameter by `Req031…` — pass
//! the sinks transposed and the compiler answers in the requirements' own
//! words.

use crate::requirements::{Req030BakedLoavesOnly, Req031ScorchedToCompost};
use crate::resources::boundary::{draw_grid_energy, draw_water};
use crate::resources::processes::{bake, divide_and_shape, draw_butter, draw_flour, draw_salt, grease_tins, knead, mix_dough, prove};
use crate::resources::{
    Atmosphere, BakeOutcome, BakedLoaf, CleanTin, FullYeastBox, Grid, Oven, PantryBag, PantryBlock,
    PantryJar, Recycling, ScorchedLoaf, Tap, UsedTin,
};
use model_core::boundary::{Consumer, send_to};
use model_core::common::Person;
use model_core::common::processes::draw_time;
use model_core::history::History;
use model_core::history::processes::record;

/// The outcome of [`bake_batch`] (R17/F-050): the four combinations of
/// SPEC.md §6 end with **different loaf counts at the household and the
/// compost stream**, so the flow's result is necessarily a four-way sum —
/// there is no single converged destination (F-046). This is a grouping over
/// sealed resources (a `pub` enum is legal for groupings, R17): holding a
/// variant means holding the two sinks after their per-combination intake.
/// Everything the combinations share ends in the one [`KitchenAtRest`]
/// returned beside this enum.
#[must_use = "BatchOutcome bundles conserved outputs: every field must be accounted for"]
pub enum BatchOutcome<H, C> {
    /// Combination 1 — both baked (SPEC.md §6): 2 `baked` loaves (744 g,
    /// 300 000 J each) delivered to the household; the compost stream empty.
    BothBaked {
        /// The household after taking both baked loaves (REQ-030).
        household: H,
        /// The compost stream, untouched: nothing scorched.
        compost: C,
    },
    /// Combination 2 — first scorched (SPEC.md §6): the first bake's loaf to
    /// compost, the second's to the household.
    FirstScorched {
        /// The household after taking the one baked loaf (REQ-030).
        household: H,
        /// The compost stream after taking the scorched first loaf (REQ-031).
        compost: C,
    },
    /// Combination 3 — second scorched (SPEC.md §6): as combination 2 with
    /// the bake order swapped.
    SecondScorched {
        /// The household after taking the one baked loaf (REQ-030).
        household: H,
        /// The compost stream after taking the scorched second loaf (REQ-031).
        compost: C,
    },
    /// Combination 4 — both scorched (SPEC.md §6): the household receives
    /// nothing; both loaves reach the compost stream.
    BothScorched {
        /// The household, untouched: nothing was fit to hand over (REQ-030).
        household: H,
        /// The compost stream after taking both scorched loaves (REQ-031).
        compost: C,
    },
}

/// Everything every combination ends with **identically** (SPEC.md §6
/// "everything accounted"): the spec's statement that all four combinations
/// draw the same time (6 300 000 ms, 7 attributed events) and energy
/// (5 000 000 J) becomes this one concrete grouping — if any combination
/// ended differently, its arm would not type-check. A grouping (R1), not a
/// sealed resource (F-046): public fields the caller destructures and
/// accounts for at the boundary.
#[must_use = "KitchenAtRest bundles conserved outputs: every field must be accounted for"]
pub struct KitchenAtRest {
    /// The baker, back with 900 000 ms of the 2-hour budget remaining.
    pub baker: Person<900_000>,
    /// The first tin, `used` at 453 g (its 3 g of butter residue, SPEC.md §8
    /// item 4).
    pub tin_1: UsedTin<453>,
    /// The second tin, `used` at 453 g.
    pub tin_2: UsedTin<453>,
    /// The oven, back in the kitchen (R2).
    pub oven: Oven,
    /// The flour bag, back as its 500 g remainder.
    pub pantry_bag: PantryBag<500>,
    /// The salt jar, back as its 482 g remainder.
    pub pantry_jar: PantryJar<482>,
    /// The butter block, back as its 238 g remainder.
    pub pantry_block: PantryBlock<238>,
    /// The tap, back at the boundary.
    pub tap: Tap,
    /// The grid, back at the boundary after the two 2 500 000 J draws.
    pub grid: Grid,
    /// The atmosphere holding both bakes' steam (200 g) and waste heat
    /// (4 400 000 J), fed inside `bake` (SPEC.md §5 P6 waste routing).
    pub atmosphere: Atmosphere,
    /// Recycling holding the 2 spent sachets and the empty yeast box
    /// (SPEC.md §5 P1 waste routing).
    pub recycling: Recycling,
    /// The single History (R16, one actor): one attributed event per draw —
    /// P1, P2, P3, P4, P5, P6, P6 — totalling 6 300 000 ms.
    pub history: History,
}

/// The full batch in order (a) of SPEC.md §6 — P1 P2 P3 P4 P5 P6 P6: mix,
/// knead, prove, grease, shape, then the two sequential bakes, each
/// consuming one provisioned outcome token, with every loaf routed to its
/// REQ-030/REQ-031 destination inside the flow and the four-combination
/// [`BatchOutcome`] returned beside the shared [`KitchenAtRest`].
#[allow(clippy::too_many_arguments)] // loose threading (R9/F-024): an aggregate would over-claim
pub fn bake_batch<H: Req030BakedLoavesOnly + Consumer<BakedLoaf, Next = H>, C: Req031ScorchedToCompost + Consumer<ScorchedLoaf, Next = C>>(
    baker: Person<7_200_000>,
    pantry_bag: PantryBag<1_500>,
    pantry_jar: PantryJar<500>,
    pantry_block: PantryBlock<250>,
    tap: Tap,
    grid: Grid,
    yeast_box: FullYeastBox,
    tin_1: CleanTin<450>,
    tin_2: CleanTin<450>,
    oven: Oven,
    first: BakeOutcome,
    second: BakeOutcome,
    household: H,
    compost: C,
    atmosphere: Atmosphere,
    recycling: Recycling,
    history: History,
) -> (BatchOutcome<H, C>, KitchenAtRest) {
    // P1 — mix the dough (900 000 ms): flour, water and salt drawn from
    // their §4 sources; taking both sachets exhausts the box.
    let (flour, pantry_bag) = draw_flour::<1_000, 500, 1_500>(pantry_bag);
    let (water, tap) = draw_water::<650>(tap);
    let (salt, pantry_jar) = draw_salt::<18, 482, 500>(pantry_jar);
    let (baker, mixed_dough, spent_1, spent_2, empty_box) =
        mix_dough(baker, flour, water, salt, yeast_box);
    let (labour, baker) = draw_time::<900_000, 6_300_000, 7_200_000>(baker);
    let history = record(history, "mix_dough", labour);
    // P1's waste is flow-routed (SPEC.md §5 P1): wrappers and box to
    // recycling.
    let recycling = send_to(recycling, spent_1);
    let recycling = send_to(recycling, spent_2);
    let recycling = send_to(recycling, empty_box);

    // P2 — knead (600 000 ms).
    let (baker, kneaded_dough) = knead(baker, mixed_dough);
    let (labour, baker) = draw_time::<600_000, 5_700_000, 6_300_000>(baker);
    let history = record(history, "knead", labour);

    // P3 — prove (3 600 000 ms) — enables REQ-028.
    let (baker, proved_dough) = prove(baker, kneaded_dough);
    let (labour, baker) = draw_time::<3_600_000, 2_100_000, 5_700_000>(baker);
    let history = record(history, "prove", labour);

    // P4 — grease the tins (120 000 ms) — enables REQ-029. Order (a) runs
    // it here; order (b) runs it first (the one ordering freedom, SPEC.md §6).
    let (butter, pantry_block) = draw_butter::<12, 238, 250>(pantry_block);
    let (baker, greased_1, greased_2) = grease_tins(baker, tin_1, tin_2, butter);
    let (labour, baker) = draw_time::<120_000, 1_980_000, 2_100_000>(baker);
    let history = record(history, "grease_tins", labour);

    // P5 — divide and shape (480 000 ms; REQ-028 on the dough).
    let (baker, shaped_1, shaped_2) = divide_and_shape(baker, proved_dough);
    let (labour, baker) = draw_time::<480_000, 1_500_000, 1_980_000>(baker);
    let history = record(history, "divide_and_shape", labour);

    // P6, first bake (300 000 ms, drawn BEFORE the branch — a scorch costs
    // it too, R17/F-048; REQ-029 on the tin). The steam and waste heat reach
    // the atmosphere inside the process (SPEC.md §5 P6).
    let (labour, baker) = draw_time::<300_000, 1_200_000, 1_500_000>(baker);
    let history = record(history, "bake", labour);
    let (energy, grid) = draw_grid_energy::<2_500_000>(grid);
    match bake(baker, oven, shaped_1, greased_1, energy, first, atmosphere) {
        Ok(ok_1) => {
            // The first loaf is fit for the household (REQ-030).
            let household = send_to(household, ok_1.baked_loaf);

            // P6, second bake: only possible with the oven the first bake
            // returned (the single-oven contention, R2/R9) and the second
            // provisioned token (F-050).
            let (labour, baker) = draw_time::<300_000, 900_000, 1_200_000>(ok_1.person);
            let history = record(history, "bake", labour);
            let (energy, grid) = draw_grid_energy::<2_500_000>(grid);
            match bake(baker, ok_1.oven, shaped_2, greased_2, energy, second, ok_1.atmosphere) {
                Ok(ok_2) => {
                    let household = send_to(household, ok_2.baked_loaf);
                    (
                        BatchOutcome::BothBaked { household, compost },
                        KitchenAtRest {
                            baker: ok_2.person,
                            tin_1: ok_1.used_tin,
                            tin_2: ok_2.used_tin,
                            oven: ok_2.oven,
                            pantry_bag,
                            pantry_jar,
                            pantry_block,
                            tap,
                            grid,
                            atmosphere: ok_2.atmosphere,
                            recycling,
                            history,
                        },
                    )
                }
                Err(fail_2) => {
                    // The second loaf is final (no rework): to compost
                    // (REQ-031).
                    let compost = send_to(compost, fail_2.scorched_loaf);
                    (
                        BatchOutcome::SecondScorched { household, compost },
                        KitchenAtRest {
                            baker: fail_2.person,
                            tin_1: ok_1.used_tin,
                            tin_2: fail_2.used_tin,
                            oven: fail_2.oven,
                            pantry_bag,
                            pantry_jar,
                            pantry_block,
                            tap,
                            grid,
                            atmosphere: fail_2.atmosphere,
                            recycling,
                            history,
                        },
                    )
                }
            }
        }
        Err(fail_1) => {
            // The first loaf is final (no rework): to compost (REQ-031).
            let compost = send_to(compost, fail_1.scorched_loaf);

            // P6, second bake — failure changed nothing about what it costs
            // (R17): same draw, same energy, the returned oven.
            let (labour, baker) = draw_time::<300_000, 900_000, 1_200_000>(fail_1.person);
            let history = record(history, "bake", labour);
            let (energy, grid) = draw_grid_energy::<2_500_000>(grid);
            match bake(baker, fail_1.oven, shaped_2, greased_2, energy, second, fail_1.atmosphere) {
                Ok(ok_2) => {
                    let household = send_to(household, ok_2.baked_loaf);
                    (
                        BatchOutcome::FirstScorched { household, compost },
                        KitchenAtRest {
                            baker: ok_2.person,
                            tin_1: fail_1.used_tin,
                            tin_2: ok_2.used_tin,
                            oven: ok_2.oven,
                            pantry_bag,
                            pantry_jar,
                            pantry_block,
                            tap,
                            grid,
                            atmosphere: ok_2.atmosphere,
                            recycling,
                            history,
                        },
                    )
                }
                Err(fail_2) => {
                    let compost = send_to(compost, fail_2.scorched_loaf);
                    (
                        BatchOutcome::BothScorched { household, compost },
                        KitchenAtRest {
                            baker: fail_2.person,
                            tin_1: fail_1.used_tin,
                            tin_2: fail_2.used_tin,
                            oven: fail_2.oven,
                            pantry_bag,
                            pantry_jar,
                            pantry_block,
                            tap,
                            grid,
                            atmosphere: fail_2.atmosphere,
                            recycling,
                            history,
                        },
                    )
                }
            }
        }
    }
}

/// The other agreed order, (b) of SPEC.md §6 — P4 P1 P2 P3 P5 P6 P6: the
/// tins are greased before the dough is even mixed (P4 is independent of
/// P1–P3, the flow's one ordering freedom), and every combination still ends
/// in the identical state — the same [`BatchOutcome`] variants and the same
/// [`KitchenAtRest`] type-check, which is the type system proving the two
/// orderings equivalent (R9).
#[allow(clippy::too_many_arguments)] // loose threading (R9/F-024): an aggregate would over-claim
pub fn bake_batch_tins_first<H: Req030BakedLoavesOnly + Consumer<BakedLoaf, Next = H>, C: Req031ScorchedToCompost + Consumer<ScorchedLoaf, Next = C>>(
    baker: Person<7_200_000>,
    pantry_bag: PantryBag<1_500>,
    pantry_jar: PantryJar<500>,
    pantry_block: PantryBlock<250>,
    tap: Tap,
    grid: Grid,
    yeast_box: FullYeastBox,
    tin_1: CleanTin<450>,
    tin_2: CleanTin<450>,
    oven: Oven,
    first: BakeOutcome,
    second: BakeOutcome,
    household: H,
    compost: C,
    atmosphere: Atmosphere,
    recycling: Recycling,
    history: History,
) -> (BatchOutcome<H, C>, KitchenAtRest) {
    // P4 — grease the tins first (120 000 ms): the ordering freedom
    // exercised (SPEC.md §6 order b).
    let (butter, pantry_block) = draw_butter::<12, 238, 250>(pantry_block);
    let (baker, greased_1, greased_2) = grease_tins(baker, tin_1, tin_2, butter);
    let (labour, baker) = draw_time::<120_000, 7_080_000, 7_200_000>(baker);
    let history = record(history, "grease_tins", labour);

    // P1 — mix the dough (900 000 ms).
    let (flour, pantry_bag) = draw_flour::<1_000, 500, 1_500>(pantry_bag);
    let (water, tap) = draw_water::<650>(tap);
    let (salt, pantry_jar) = draw_salt::<18, 482, 500>(pantry_jar);
    let (baker, mixed_dough, spent_1, spent_2, empty_box) =
        mix_dough(baker, flour, water, salt, yeast_box);
    let (labour, baker) = draw_time::<900_000, 6_180_000, 7_080_000>(baker);
    let history = record(history, "mix_dough", labour);
    let recycling = send_to(recycling, spent_1);
    let recycling = send_to(recycling, spent_2);
    let recycling = send_to(recycling, empty_box);

    // P2 — knead (600 000 ms).
    let (baker, kneaded_dough) = knead(baker, mixed_dough);
    let (labour, baker) = draw_time::<600_000, 5_580_000, 6_180_000>(baker);
    let history = record(history, "knead", labour);

    // P3 — prove (3 600 000 ms).
    let (baker, proved_dough) = prove(baker, kneaded_dough);
    let (labour, baker) = draw_time::<3_600_000, 1_980_000, 5_580_000>(baker);
    let history = record(history, "prove", labour);

    // P5 — divide and shape (480 000 ms; REQ-028 on the dough). From here
    // the orders converge: both reach this point at 1 500 000 ms left.
    let (baker, shaped_1, shaped_2) = divide_and_shape(baker, proved_dough);
    let (labour, baker) = draw_time::<480_000, 1_500_000, 1_980_000>(baker);
    let history = record(history, "divide_and_shape", labour);

    // P6, first bake (drawn before the branch, R17/F-048).
    let (labour, baker) = draw_time::<300_000, 1_200_000, 1_500_000>(baker);
    let history = record(history, "bake", labour);
    let (energy, grid) = draw_grid_energy::<2_500_000>(grid);
    match bake(baker, oven, shaped_1, greased_1, energy, first, atmosphere) {
        Ok(ok_1) => {
            let household = send_to(household, ok_1.baked_loaf);
            let (labour, baker) = draw_time::<300_000, 900_000, 1_200_000>(ok_1.person);
            let history = record(history, "bake", labour);
            let (energy, grid) = draw_grid_energy::<2_500_000>(grid);
            match bake(baker, ok_1.oven, shaped_2, greased_2, energy, second, ok_1.atmosphere) {
                Ok(ok_2) => {
                    let household = send_to(household, ok_2.baked_loaf);
                    (
                        BatchOutcome::BothBaked { household, compost },
                        KitchenAtRest {
                            baker: ok_2.person,
                            tin_1: ok_1.used_tin,
                            tin_2: ok_2.used_tin,
                            oven: ok_2.oven,
                            pantry_bag,
                            pantry_jar,
                            pantry_block,
                            tap,
                            grid,
                            atmosphere: ok_2.atmosphere,
                            recycling,
                            history,
                        },
                    )
                }
                Err(fail_2) => {
                    let compost = send_to(compost, fail_2.scorched_loaf);
                    (
                        BatchOutcome::SecondScorched { household, compost },
                        KitchenAtRest {
                            baker: fail_2.person,
                            tin_1: ok_1.used_tin,
                            tin_2: fail_2.used_tin,
                            oven: fail_2.oven,
                            pantry_bag,
                            pantry_jar,
                            pantry_block,
                            tap,
                            grid,
                            atmosphere: fail_2.atmosphere,
                            recycling,
                            history,
                        },
                    )
                }
            }
        }
        Err(fail_1) => {
            let compost = send_to(compost, fail_1.scorched_loaf);
            let (labour, baker) = draw_time::<300_000, 900_000, 1_200_000>(fail_1.person);
            let history = record(history, "bake", labour);
            let (energy, grid) = draw_grid_energy::<2_500_000>(grid);
            match bake(baker, fail_1.oven, shaped_2, greased_2, energy, second, fail_1.atmosphere) {
                Ok(ok_2) => {
                    let household = send_to(household, ok_2.baked_loaf);
                    (
                        BatchOutcome::FirstScorched { household, compost },
                        KitchenAtRest {
                            baker: ok_2.person,
                            tin_1: fail_1.used_tin,
                            tin_2: ok_2.used_tin,
                            oven: ok_2.oven,
                            pantry_bag,
                            pantry_jar,
                            pantry_block,
                            tap,
                            grid,
                            atmosphere: ok_2.atmosphere,
                            recycling,
                            history,
                        },
                    )
                }
                Err(fail_2) => {
                    let compost = send_to(compost, fail_2.scorched_loaf);
                    (
                        BatchOutcome::BothScorched { household, compost },
                        KitchenAtRest {
                            baker: fail_2.person,
                            tin_1: fail_1.used_tin,
                            tin_2: fail_2.used_tin,
                            oven: fail_2.oven,
                            pantry_bag,
                            pantry_jar,
                            pantry_block,
                            tap,
                            grid,
                            atmosphere: fail_2.atmosphere,
                            recycling,
                            history,
                        },
                    )
                }
            }
        }
    }
}
