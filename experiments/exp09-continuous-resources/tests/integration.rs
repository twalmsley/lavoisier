//! EXP-09 integration tests: the full flow in two valid orders (R9), the
//! boundary-trait probes with `Next = Self`, budget threading, and the
//! tripwire demonstrations for the two "silently lost" cases.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(let_underscore_drop)]

use exp09_continuous_resources::boundary_traits::{
    send_list, send_to, source, take_n, Cons, Nil, Succ, Zero,
};
use exp09_continuous_resources::model::boundary::{
    draw_air, fill_gas_bottle, new_depot, new_ledger, new_person, new_work_sink, the_atmosphere,
};
use exp09_continuous_resources::model::processes::{
    burn, combine_air, draw_gas, draw_time, inspect_heat, walk_to_station,
};
use exp09_continuous_resources::model::{
    Air, Atmosphere, Depot, EmptyGasBottle, Gas, GasBottle, Ledger, Person, WorkSink,
};

type N3 = Succ<Succ<Succ<Zero>>>;

/// Order A: draw gas, then air, then time; burn; dispose exhaust before heat.
/// Returns the part-empty bottle and the part-spent person (R15 integration).
fn flow_gas_first() -> (GasBottle<4700>, Person<5000>, Atmosphere, Ledger, WorkSink) {
    let bottle = fill_gas_bottle::<5000>();
    let person = new_person::<10000>();
    let atm = the_atmosphere();
    let ledger = new_ledger();
    let sink = new_work_sink();

    let (gas, bottle) = draw_gas::<300, 4700, 5000>(bottle);
    let (air, atm) = draw_air::<300>(atm);
    let (labour, person) = draw_time::<2000, 8000, 10000>(person); // set-up time
    let ledger = send_to(ledger, labour);

    let (exhaust, heat, work) = burn::<300, 300, 600, 11000, 4000>(gas, air);

    let (labour, person) = draw_time::<3000, 5000, 8000>(person); // operating time
    let ledger = send_to(ledger, labour);

    let atm = send_to(atm, exhaust);
    let atm = send_to(atm, heat);
    let sink = send_to(sink, work);

    (bottle, person, atm, ledger, sink)
}

/// Order B: the same flow with the independent steps reordered — time and air
/// first, gas last before burning; heat disposed of before exhaust; the person
/// also passes through a process that spends no time (`walk_to_station`).
fn flow_air_first() -> (GasBottle<4700>, Person<5000>, Atmosphere, Ledger, WorkSink) {
    let person = new_person::<10000>();
    let atm = the_atmosphere();

    let (labour, person) = draw_time::<2000, 8000, 10000>(person);
    let person = walk_to_station(person);
    let (air, atm) = draw_air::<300>(atm);

    let bottle = fill_gas_bottle::<5000>();
    let (gas, bottle) = draw_gas::<300, 4700, 5000>(bottle);

    let (exhaust, heat, work) = burn::<300, 300, 600, 11000, 4000>(gas, air);

    let atm = send_to(atm, heat);
    let atm = send_to(atm, exhaust);

    let ledger = send_to(new_ledger(), labour);
    let (labour, person) = draw_time::<3000, 5000, 8000>(person);
    let ledger = send_to(ledger, labour);
    let sink = send_to(new_work_sink(), work);

    (bottle, person, atm, ledger, sink)
}

fn dispose(bottle: GasBottle<4700>, person: Person<5000>) -> (Depot, Person<5000>) {
    // The part-empty bottle is accounted for at the boundary; the part-spent
    // person (a reusable resource) stays with the caller.
    (send_to(new_depot(), bottle), person)
}

/// Both orders type-check and run: R9 sequencing freedom holds for the
/// continuous-resource flow.
#[test]
fn integration_flow_order_a() {
    let (bottle, person, _atm, _ledger, _sink) = flow_gas_first();
    assert_eq!(GasBottle::<4700>::VALUE, 4700);
    assert_eq!(Person::<5000>::BUDGET_MS, 5000);
    let (_depot, _person) = dispose(bottle, person);
}

#[test]
fn integration_flow_order_b() {
    let (bottle, person, _atm, _ledger, _sink) = flow_air_first();
    let (_depot, _person) = dispose(bottle, person);
}

/// The person's budget is drawn down across several calls; the budget const
/// parameter appears in every signature the person passes through (see
/// `walk_to_station` in the library and the RESULTS.md ergonomics notes).
#[test]
fn time_budget_draws_down_across_a_flow() {
    let person = new_person::<10000>();
    let (l1, person) = draw_time::<2000, 8000, 10000>(person);
    let person = walk_to_station(person); // no spend: Person<8000> in and out
    let (l2, person) = draw_time::<3000, 5000, 8000>(person);
    let (l3, person) = draw_time::<5000, 0, 5000>(person); // budget fully spent
    let ledger = send_to(new_ledger(), l1);
    let ledger = send_to(ledger, l2);
    let _ledger = send_to(ledger, l3);
    assert_eq!(Person::<0>::BUDGET_MS, 0); // Person<0> is the spent state
    let _spent_person = person;
}

/// Step 5 probe (a): fixed-packet supply from the unbounded atmosphere via the
/// discrete `Supplier` trait + `SupplyN`, with `Next = Self`; the packets then
/// need explicit `combine_air` processes to become one usable amount.
#[test]
fn fixed_packet_air_supply_via_supply_n() {
    let atm = the_atmosphere();
    // Three fixed 100 g packets; SupplyN tolerates Next = Self.
    let (taken, atm) = take_n::<N3, _>(atm);
    let Cons(a1, Cons(a2, Cons(a3, Nil))) = taken;
    let air = combine_air::<100, 100, 200>(a1, a2);
    let air = combine_air::<200, 100, 300>(air, a3);

    // Use the combined air so everything is accounted for.
    let bottle = fill_gas_bottle::<300>();
    let (gas, empty) = draw_gas::<300, 0, 300>(bottle);
    let (exhaust, heat, work) = burn::<300, 300, 600, 11000, 4000>(gas, air);
    let atm = send_to(atm, exhaust);
    let _atm = send_to(atm, heat);
    let _sink = send_to(new_work_sink(), work);
    let empty: EmptyGasBottle = empty; // the empty state is a distinct, named type
    let _depot = send_to(new_depot(), empty);
}

/// Step 5 probe (c): the parameterized `SupplierOf<Out>` lets the caller pick
/// the amount through an annotation — draw-style supply in trait clothing.
#[test]
fn parameterized_supplier_of_draws_a_chosen_amount() {
    let atm = the_atmosphere();
    let (air, atm): (Air<300>, _) = source(atm);
    assert_eq!(Air::<300>::VALUE, 300);

    let bottle = fill_gas_bottle::<300>();
    let (gas, empty) = draw_gas::<300, 0, 300>(bottle);
    let (exhaust, heat, work) = burn::<300, 300, 600, 11000, 4000>(gas, air);
    let atm = send_to(atm, exhaust);
    let _atm = send_to(atm, heat);
    let _sink = send_to(new_work_sink(), work);
    let _depot = send_to(new_depot(), empty);
}

/// Step 5: one boundary object consumes two different things (exhaust and
/// heat), and `ConsumeList` repeated use tolerates `Next = Self`.
#[test]
fn atmosphere_consumes_exhaust_and_heat_as_a_list() {
    let atm = the_atmosphere();
    let (air, atm) = draw_air::<300>(atm);
    let bottle = fill_gas_bottle::<300>();
    let (gas, empty) = draw_gas::<300, 0, 300>(bottle);
    let (exhaust, heat, work) = burn::<300, 300, 600, 11000, 4000>(gas, air);
    // A heterogeneous list of two different waste outputs into one consumer.
    let _atm = send_list(atm, Cons(exhaust, Cons(heat, Nil)));
    let _sink = send_to(new_work_sink(), work);
    let _depot = send_to(new_depot(), empty);
}

/// Value-level conservation check backing the type-level one (R5): the
/// associated VALUE constants balance per dimension.
#[test]
fn burn_balances_mass_and_energy_at_the_value_level() {
    // mass: fuel + air = exhaust
    assert_eq!(Gas::<300>::VALUE + Air::<300>::VALUE, 600);
    // energy: fuel energy = heat + work
    assert_eq!(
        Gas::<300>::VALUE * exp09_continuous_resources::model::GAS_ENERGY_J_PER_G,
        11000 + 4000
    );
}

/// The "empty bottle dropped silently" case (EXP-09 step 6). No compile-time
/// layer catches it: the binding is named and used (`draw_gas` consumed the
/// full bottle; `empty` is its successor and is simply abandoned). The
/// tripwire `Drop` catches it at test time — this is the layer that fires.
#[test]
#[should_panic(expected = "resource leak: GasBottle<0>")]
fn empty_bottle_dropped_silently_trips_the_drop_tripwire() {
    let bottle = fill_gas_bottle::<300>();
    let (gas, empty) = draw_gas::<300, 0, 300>(bottle);
    let atm = the_atmosphere();
    let (air, atm) = draw_air::<300>(atm);
    let (exhaust, heat, work) = burn::<300, 300, 600, 11000, 4000>(gas, air);
    let atm = send_to(atm, exhaust);
    let _atm = send_to(atm, heat);
    let _sink = send_to(new_work_sink(), work);
    // `empty` (a GasBottle<0> — still a resource, R15) goes out of scope here
    // without reaching a consumer: the tripwire panics.
    let _still_bound_but_never_consumed = empty;
}

/// Waste heat that is used once and then dropped: invisible to `must_use`,
/// `unused_variables` and every other compile-time layer (F-002's
/// used-then-dropped gap); only the tripwire catches it.
#[test]
#[should_panic(expected = "resource leak: WasteHeat<11000>")]
fn waste_heat_used_then_dropped_trips_the_drop_tripwire() {
    let bottle = fill_gas_bottle::<300>();
    let (gas, empty) = draw_gas::<300, 0, 300>(bottle);
    let atm = the_atmosphere();
    let (air, atm) = draw_air::<300>(atm);
    let (exhaust, heat, work) = burn::<300, 300, 600, 11000, 4000>(gas, air);
    let _atm = send_to(atm, exhaust);
    let _sink = send_to(new_work_sink(), work);
    let _depot = send_to(new_depot(), empty);
    // "Used" once — every lint is now satisfied — then silently dropped.
    let heat = inspect_heat(heat);
    let _still_bound_but_never_consumed = heat;
}
