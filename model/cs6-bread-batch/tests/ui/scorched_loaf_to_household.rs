// REQ-030: the household accepts only baked loaves — a scorched loaf may
// not be handed over. The household's `Consumer` impl exists for the baked
// state alone (REQ-030 structural), so this is a type-check-time E0277 with
// model-core's consumer phrasing; the scorched loaf's only exit is the
// compost stream (REQ-031).
use cs6_bread_batch::resources::ScorchedLoaf;
use cs6_bread_batch::resources::boundary::new_household;
use model_core::boundary::send_to;

fn main() {
    let scorched = ScorchedLoaf::test_fixture();
    // The household must not take it: this must not compile.
    let household = send_to(new_household(), scorched);
}
