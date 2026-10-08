//! File emission: the generated crate's Cargo.toml, src/*.rs and
//! tests/flows.rs, built from the [`Generator`]'s resolved plans.
//!
//! The generation regime (R22): deterministic and idempotent (two runs are
//! byte-identical); every emitted item carries a breadcrumb to its SPEC.md
//! §/line, placed in rustdoc or on the code line so Rust diagnostics render
//! it (F-061); anything the spec under-determines is an enumerable
//! SPEC-HOLE, never a guess. The output is ONE-SHOT scaffolding: it is
//! promoted to hand-maintained in the same change that commits it.

use super::*;

/// A process signature shared by the definition emitter and the flow
/// emitter, so calls always match definitions.
pub struct Sig {
    pub generics: Vec<String>,
    /// (var/param name, type)
    pub params: Vec<(String, String)>,
    /// (result var name, type) — tuple order
    pub returns: Vec<(String, String)>,
}

fn ty_of(item: &Item) -> String {
    if item.consts.is_empty() {
        item.ident.clone()
    } else {
        format!(
            "{}<{}>",
            item.ident,
            item.consts
                .iter()
                .map(|c| lit(*c))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn cons_list(item: &str, n: u64) -> String {
    let mut s = "Nil".to_string();
    for _ in 0..n {
        s = format!("Cons<{item}, {s}>");
    }
    s
}

fn held_of(g: &Generator, ident: &str) -> Option<(u64, String)> {
    g.lexicon
        .iter()
        .find(|e| e.ident == ident && !e.alias_only)
        .and_then(|e| match &e.role {
            Role::Consumable { held, .. } => held.clone(),
            _ => None,
        })
}

fn is_container(g: &Generator, ident: &str) -> bool {
    g.lexicon
        .iter()
        .chain(g.synthesized.iter())
        .any(|e| e.ident == ident && !e.alias_only && matches!(e.role, Role::Container { .. }))
}

/// Untripwired consumable (F-040): it has no `defuse`, so a conserving
/// transform consumes it by destructuring (the CS-2 `DryPatch` shape).
fn is_untripwired(g: &Generator, ident: &str) -> bool {
    g.lexicon.iter().any(|e| {
        e.ident == ident
            && !e.alias_only
            && matches!(e.role, Role::Consumable { tripwire: false, .. })
    })
}

/// The R17 dual-arm shape is scaffoldable when the process states both arms
/// and consumes exactly one outcome token (and no SupplyN rides along — that
/// interaction is not derivable and becomes a hole instead).
fn fallible(plan: &ProcPlan) -> bool {
    plan.outcome.is_some()
        && !plan.produces_ok.is_empty()
        && !plan.produces_fail.is_empty()
        && plan.supply_n.is_none()
}

/// Bundle names, mechanically from the process name (F-062): `bake` →
/// (`BakeOk`, `BakeFail`).
fn bundle_names(plan: &ProcPlan) -> (String, String) {
    let base = camel(&plan.fn_name);
    (format!("{base}Ok"), format!("{base}Fail"))
}

/// One arm bundle's fields, in deterministic order: the person, the arm's
/// products, the returned reusables, the waste outputs. Shared by the bundle
/// struct emitter, the process body and the flow emitter, so the three always
/// agree.
fn bundle_fields(plan: &ProcPlan, ok: bool) -> Vec<(String, String)> {
    let mut fields: Vec<(String, String)> = Vec::new();
    if plan.person {
        fields.push(("person".into(), "Person<B>".into()));
    }
    let items = if ok { &plan.produces_ok } else { &plan.produces_fail };
    let push_item = |fields: &mut Vec<(String, String)>, it: &Item| {
        if it.count > 1 {
            for i in 1..=it.count {
                fields.push((format!("{}_{i}", snake(&it.ident)), ty_of(it)));
            }
        } else {
            fields.push((snake(&it.ident), ty_of(it)));
        }
    };
    for it in items {
        push_item(&mut fields, it);
    }
    for r in &plan.reusable_returns {
        fields.push((snake(r), r.clone()));
    }
    for w in &plan.waste {
        if w.duplicate_of_produce {
            continue;
        }
        if let Some(it) = &w.item {
            push_item(&mut fields, it);
        }
    }
    fields
}

pub fn build_sig(g: &Generator, plan: &ProcPlan) -> Sig {
    let mut sig = Sig {
        generics: Vec::new(),
        params: Vec::new(),
        returns: Vec::new(),
    };
    if plan.person {
        sig.generics.push("const B: u64".into());
        sig.params.push(("person".into(), "Person<B>".into()));
        if !fallible(plan) {
            // in the R17 dual-arm shape the person rides the bundles instead
            sig.returns.push(("person".into(), "Person<B>".into()));
        }
    }
    if let Some(d) = &plan.bin_disposal {
        let list = cons_list(&d.item, d.count);
        sig.params.push((
            snake(&d.bin_ident),
            format!("{}<N{}, {}>", d.bin_ident, d.cap - d.count, list),
        ));
        sig.params.push((snake(&d.sink_ident), d.sink_ident.clone()));
        sig.returns.push((
            snake(&d.bin_ident),
            format!("{}<N{}>", d.bin_ident, d.cap),
        ));
        sig.returns.push((snake(&d.sink_ident), d.sink_ident.clone()));
        return sig;
    }
    for r in &plan.reusable_params {
        sig.params.push((snake(r), r.clone()));
    }
    for c in &plan.consumes {
        if plan
            .supply_n
            .as_ref()
            .map(|(_, item)| item == &c.ident)
            .unwrap_or(false)
        {
            continue; // supplied via SupplyN
        }
        if c.count > 1 {
            // N consumed items are N by-value parameters (D2/D3): a consumed
            // input may never vanish from the signature
            for i in 1..=c.count {
                sig.params.push((format!("{}_{i}", snake(&c.ident)), ty_of(c)));
            }
        } else {
            sig.params.push((snake(&c.ident), ty_of(c)));
        }
    }
    let mut supplier_var = None;
    if let Some((n, item)) = &plan.supply_n {
        if let Some(sp) = g.suppliers.iter().find(|s| &s.item == item) {
            // a supplier whose container is itself consumed is pinned to its
            // exhausted state, so the §3 `empty` state can continue from it
            let rest = if plan.supplier_consumed {
                format!(", Rest = Empty{}", sp.ident)
            } else {
                String::new()
            };
            sig.generics.push(format!(
                "S: SupplyN<N{n}, Taken = {}{rest}>",
                cons_list(item, *n)
            ));
            supplier_var = Some(snake(&sp.ident));
            sig.params.push((supplier_var.clone().unwrap(), "S".into()));
        }
    }
    if fallible(plan) {
        // The R17 dual-arm shape (the CS-2 patch_tube pattern): one Result,
        // each arm a bundle carrying that arm's products plus every conserved
        // participant.
        let (ok_name, fail_name) = bundle_names(plan);
        let b = if plan.person { "<B>" } else { "" };
        sig.returns.push((
            "attempt".into(),
            format!("Result<{ok_name}{b}, {fail_name}{b}>"),
        ));
        return sig;
    }
    for p in &plan.produces {
        if p.count > 1 {
            for i in 1..=p.count {
                sig.returns.push((format!("{}_{i}", snake(&p.ident)), ty_of(p)));
            }
        } else {
            sig.returns.push((snake(&p.ident), ty_of(p)));
        }
    }
    for r in &plan.reusable_returns {
        sig.returns.push((snake(r), r.clone()));
    }
    if let Some(v) = supplier_var {
        if !plan.supplier_consumed {
            sig.returns.push((v, "S::Rest".into()));
        }
    }
    for w in &plan.waste {
        if w.duplicate_of_produce {
            continue;
        }
        if let Some(it) = &w.item {
            if it.count > 1 {
                for i in 1..=it.count {
                    sig.returns.push((format!("{}_{i}", snake(&it.ident)), ty_of(it)));
                }
            } else {
                sig.returns.push((snake(&it.ident), ty_of(it)));
            }
        }
    }
    sig
}

fn hole_block(g: &mut Generator, indent: &str, summary: &str, detail: &str) -> String {
    let id = g.hole(summary, detail);
    let mut s = String::new();
    let _ = writeln!(
        s,
        "{indent}// ── SPEC-HOLE U-{id:02} ──────────────────────────────────────────"
    );
    for l in detail.lines() {
        let _ = writeln!(s, "{indent}// {l}");
    }
    let _ = writeln!(s, "{indent}#[cfg(feature = \"deny-holes\")]");
    let _ = writeln!(s, "{indent}compile_error!(\"SPEC-HOLE U-{id:02}: {summary}\");");
    s
}

fn req_trait_name(r: &Requirement) -> String {
    let sentence = strip_parens(&r.text);
    let ws: Vec<String> = words(&sentence).into_iter().take(4).collect();
    format!("Req{:03}{}", r.impl_id, camel(&ws.join(" ")))
}

fn clean(s: &str) -> String {
    s.replace('"', "'").replace('`', "'")
}

// ── per-file emitters ────────────────────────────────────────────────────

fn emit_cargo_toml(pkg: &str, model_core: &str) -> String {
    format!(
        r#"# GENERATED by tools/spec-gen (specgen, R22) from SPEC.md — one-shot
# scaffolding: hand-maintained from the moment it is committed.
# `cargo build` compiles everything the spec determines;
# `cargo build --features deny-holes` turns every SPEC-HOLE into a compile error.

[package]
name = "{pkg}"
version = "0.1.0"
edition = "2024"

[features]
deny-holes = []
# Declared because the model-core kernel macros expand a `test_fixture()`
# constructor behind the INVOKING crate's `test-support` feature (F-004).
test-support = []

[dependencies]
model-core = {{ path = "{model_core}" }}

[workspace]
"#
    )
}

fn emit_lib(g: &mut Generator, spec_name: &str) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "//! GENERATED from the agreed specification \"{spec_name}\" by"
    );
    let _ = writeln!(s, "//! tools/spec-gen (specgen, R22) — ONE-SHOT scaffolding, promoted to");
    let _ = writeln!(s, "//! hand-maintained in the same change that commits it. Every item carries");
    let _ = writeln!(s, "//! a breadcrumb to the SPEC.md section/line it was recovered from; every");
    let _ = writeln!(s, "//! under-determined decision is a numbered SPEC-HOLE (see Cargo.toml).");
    let _ = writeln!(s);
    let _ = writeln!(s, "#![recursion_limit = \"2048\"]");
    let _ = writeln!(s, "#![forbid(unsafe_code)]");
    let _ = writeln!(s, "#![deny(unused_must_use)]");
    let _ = writeln!(s, "#![deny(let_underscore_drop)]");
    let _ = writeln!(s);
    s.push_str(&hole_block(
        g,
        "",
        "per-process unit tests (R5) and compile-fail regressions (R4) are not derivable from the spec",
        "The spec's §5 blocks determine signatures and balances, but not the R5 test\nfixtures, the R4 trybuild cases, or the rustdoc compile_fail regressions the\nreal crate carries — all are implementation judgement.",
    ));
    let _ = writeln!(s);
    let _ = writeln!(s, "pub mod characteristics;");
    let _ = writeln!(s, "pub mod requirements;");
    let _ = writeln!(s, "pub mod resources;");
    s
}

fn emit_characteristics(g: &mut Generator) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "//! GENERATED characteristic markers (R6) — one synthesized marker per §2\n//! requirement. The spec names requirements but not the characteristic\n//! machinery behind them, so each marker is a hole."
    );
    let _ = writeln!(s);
    let reqs = g.spec.requirements.clone();
    for r in &reqs {
        let marker = format!("Req{:03}Subject", r.impl_id);
        let _ = writeln!(
            s,
            "/// Synthesized characteristic for REQ-{:03} (SPEC.md §2, line {}).",
            r.impl_id, r.line
        );
        s.push_str(&hole_block(
            g,
            "",
            &format!(
                "no §3 column says which resource state carries the REQ-{:03} characteristic",
                r.impl_id
            ),
            &format!(
                "REQ-{:03} (\"{}\", SPEC.md §2 line {}) becomes a requirement trait, but the\nspec does not state which type implements its characteristic, what quantities\nthe characteristic carries, or the sealed/permit-gated extraction design.\nAttach `{marker}` to the right state type by hand.",
                r.impl_id,
                clean(&r.text),
                r.line
            ),
        ));
        let _ = writeln!(
            s,
            "#[diagnostic::on_unimplemented(message = \"`{{Self}}` does not satisfy REQ-{:03}: {}\")]",
            r.impl_id,
            clean(&strip_parens(&r.text)).trim()
        );
        let _ = writeln!(s, "pub trait {marker} {{}}");
        let _ = writeln!(s);
    }
    s
}

fn emit_requirements(g: &mut Generator) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "//! GENERATED requirement traits (R10) from SPEC.md §2, via\n//! `model_core::requirement!` (blanket impl + per-requirement assert fn,\n//! with a REQ-phrased `#[diagnostic::on_unimplemented]`, R10 rule 8)."
    );
    let _ = writeln!(s);
    let reqs = g.spec.requirements.clone();
    let markers: Vec<String> = reqs
        .iter()
        .map(|r| format!("Req{:03}Subject", r.impl_id))
        .collect();
    let _ = writeln!(s, "use crate::characteristics::{{{}}};", markers.join(", "));
    let _ = writeln!(s);
    s.push_str(&hole_block(
        g,
        "",
        "`Satisfies:` doc tags and their backing `satisfies!` assertions cannot be placed",
        "The spec says which PROCESSES satisfy which requirements (§5 Satisfies lines,\nemitted as doc tags on the process fns), but not which TYPES do, nor the\nalias-carries-the-tag placement the real crates use to dodge F-037. The\n`satisfies!` compile-checked assertions therefore cannot be generated.",
    ));
    let _ = writeln!(s);
    for r in &reqs {
        let name = req_trait_name(r);
        let marker = format!("Req{:03}Subject", r.impl_id);
        let sentence = clean(&r.text);
        let plain = clean(&strip_parens(&r.text));
        let _ = writeln!(s, "model_core::requirement! {{");
        let _ = writeln!(s, "    /// REQ-{:03}: {}", r.impl_id, sentence);
        if r.impl_id != r.spec_id {
            let _ = writeln!(
                s,
                "    /// (SPEC.md §2 line {} numbers this requirement REQ-{:03}; ids remapped per the §2 implementation note, F-053.)",
                r.line, r.spec_id
            );
        }
        let _ = writeln!(
            s,
            "    #[diagnostic::on_unimplemented(message = \"`{{Self}}` does not satisfy REQ-{:03}: {}\", label = \"REQ-{:03}: {}\")]",
            r.impl_id,
            plain.trim(),
            r.impl_id,
            plain.trim()
        );
        let _ = writeln!(s, "    pub trait {name}: ({marker});");
        let _ = writeln!(s, "    assert = assert_req{:03};", r.impl_id);
        let _ = writeln!(s, "}}");
        let _ = writeln!(s);
    }
    s
}

fn must_use_for(ident: &str, role: &Role) -> String {
    match role {
        Role::Container { .. } | Role::Consumable { .. } => {
            format!("{ident} is a conserved resource: pass it on or hand it to a Consumer")
        }
        Role::Reusable => {
            format!("{ident} is a reusable resource: pass it on or return it to the caller")
        }
        Role::Outcome => format!(
            "{ident} is a boundary token: run it through exactly one fallible process or return it to the environment"
        ),
        Role::Common => String::new(),
    }
}

fn emit_resource_types(g: &mut Generator, s: &mut String) {
    let entries: Vec<TypeEntry> = g
        .lexicon
        .iter()
        .cloned()
        .chain(g.synthesized.iter().cloned())
        .collect();
    let mut seen: Vec<String> = Vec::new();
    for e in entries {
        if matches!(e.role, Role::Common) || e.alias_only || seen.contains(&e.ident) {
            continue;
        }
        seen.push(e.ident.clone());
        let breadcrumb = format!(
            "SPEC.md §3 line {}: resource '{}'{}",
            e.line,
            clean(&e.resource),
            e.state
                .as_ref()
                .map(|st| format!(", state '{st}'"))
                .unwrap_or_default()
        );
        match &e.role {
            Role::Container {
                extra_units,
                slot_unit,
            } => {
                let _ = writeln!(s, "model_core::container_resource! {{");
                let _ = writeln!(s, "    /// Generated from {breadcrumb}.");
                if extra_units.is_empty() {
                    let _ = writeln!(s, "    {},", e.ident);
                } else {
                    let params = extra_units
                        .iter()
                        .map(|u| format!("const {}: u64", const_param_name(u)))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let _ = writeln!(s, "    {}<{}>,", e.ident, params);
                }
                let _ = writeln!(s, "    unit = \"{}\",", unit_name(slot_unit));
                let _ = writeln!(s, "    must_use = \"{}\"", must_use_for(&e.ident, &e.role));
                let _ = writeln!(s, "}}");
                let _ = writeln!(s);
            }
            Role::Consumable {
                held,
                tripwire,
                mass_g,
            } => {
                let _ = writeln!(s, "model_core::consumable_resource! {{");
                let _ = writeln!(s, "    /// Generated from {breadcrumb}.");
                if !*tripwire {
                    let _ = writeln!(
                        s,
                        "    /// `no_tripwire`: minted into a boundary container that keeps it (the F-040 rule)."
                    );
                }
                match held {
                    Some((n, item)) => {
                        let tuple = (0..*n).map(|_| item.as_str()).collect::<Vec<_>>().join(", ");
                        let _ = writeln!(s, "    {} {{", e.ident);
                        let _ = writeln!(s, "        held: ({tuple}),");
                        let _ = writeln!(s, "    }},");
                    }
                    None => {
                        let _ = writeln!(s, "    {},", e.ident);
                    }
                }
                if *tripwire {
                    let _ = writeln!(s, "    must_use = \"{}\"", must_use_for(&e.ident, &e.role));
                } else {
                    let _ = writeln!(s, "    must_use = \"{}\",", must_use_for(&e.ident, &e.role));
                    let _ = writeln!(s, "    no_tripwire");
                }
                let _ = writeln!(s, "}}");
                let _ = writeln!(s);
                if let Some(m) = mass_g {
                    let _ = writeln!(s, "impl {} {{", e.ident);
                    let _ = writeln!(
                        s,
                        "    /// Mass in grams (R7), from the §3 characteristics column (line {}).",
                        e.line
                    );
                    let _ = writeln!(s, "    pub const MASS_G: u64 = {};", lit(*m));
                    let _ = writeln!(s, "}}");
                    let _ = writeln!(s);
                }
            }
            Role::Reusable => {
                let _ = writeln!(s, "model_core::reusable_resource! {{");
                let _ = writeln!(s, "    /// Generated from {breadcrumb}.");
                let _ = writeln!(s, "    {},", e.ident);
                let _ = writeln!(s, "    must_use = \"{}\"", must_use_for(&e.ident, &e.role));
                let _ = writeln!(s, "}}");
                let _ = writeln!(s);
            }
            Role::Outcome => {
                let sn = snake(&e.ident);
                let _ = writeln!(s, "model_core::outcome_token! {{");
                let _ = writeln!(s, "    /// Generated from {breadcrumb} — one trial of the");
                let _ = writeln!(s, "    /// environment (R17, F-042): sealed, injected only at the boundary (the");
                let _ = writeln!(s, "    /// constructors below stay placeholders until calibrated), realised by");
                let _ = writeln!(s, "    /// exactly one fallible process. A flow cannot read it: the only way to");
                let _ = writeln!(s, "    /// learn the outcome is to run the process and handle both `Result` arms.");
                let _ = writeln!(s, "    {}({}Kind),", e.ident, e.ident);
                let _ = writeln!(s, "    success = {sn}_will_succeed,");
                let _ = writeln!(s, "    failure = {sn}_will_fail,");
                let _ = writeln!(s, "    exit = return_{sn},");
                let _ = writeln!(s, "    must_use = \"{}\"", must_use_for(&e.ident, &e.role));
                let _ = writeln!(s, "}}");
                let _ = writeln!(s);
            }
            Role::Common => {}
        }
    }
    // draw-source boundary objects (reusable, from §4)
    let draws = g.draws.clone();
    for d in &draws {
        let _ = writeln!(s, "model_core::reusable_resource! {{");
        let _ = writeln!(
            s,
            "    /// Generated from SPEC.md §4 line {}: boundary source drawn via [`boundary::{}`].",
            d.line, d.fn_name
        );
        let _ = writeln!(s, "    ///");
        let _ = writeln!(s, "    /// Placeholder: unbounded boundary source (R15).");
        let _ = writeln!(s, "    {},", d.obj_ident);
        let _ = writeln!(
            s,
            "    must_use = \"{} is a boundary resource: pass it on like any other resource\"",
            d.obj_ident
        );
        let _ = writeln!(s, "}}");
        let _ = writeln!(s);
    }
    // bounded continuous sources with remainder (the pantry containers, §4):
    // a container type drawn down by an R15 draw process in `boundary`
    let bags = g.bags.clone();
    for b in &bags {
        let _ = writeln!(s, "model_core::container_resource! {{");
        let _ = writeln!(
            s,
            "    /// Generated from SPEC.md §4 line {}: a bounded source of {} with",
            b.line, b.item
        );
        let _ = writeln!(
            s,
            "    /// remainder — enters full (capacity {}), drawn down via [`boundary::{}`],"
            , lit(b.capacity), b.fn_name
        );
        let _ = writeln!(s, "    /// and its remainder is accounted at flow end (SPEC.md §6).");
        let _ = writeln!(s, "    {},", b.obj_ident);
        let _ = writeln!(s, "    unit = \"{} remaining\",", unit_name(&b.slot_unit));
        let _ = writeln!(
            s,
            "    must_use = \"{} is a conserved resource: even an exhausted container must be accounted for\"",
            b.obj_ident
        );
        let _ = writeln!(s, "}}");
        let _ = writeln!(s);
    }
}

/// The R17 arm bundles (the CS-2 `PatchOk`/`PatchFail` shape): a grouping
/// (R1), not a sealed resource (F-046) — public fields, buildable only from
/// already-held resources, `#[must_use]`, and deliberately **no `Debug`**
/// (F-047) so `.unwrap()`/`.expect()` on the process `Result` cannot compile.
fn emit_bundles(g: &mut Generator, s: &mut String) {
    let plans = g.plans.clone();
    for plan in &plans {
        if !fallible(plan) {
            continue;
        }
        let (ok_name, fail_name) = bundle_names(plan);
        for (name, ok) in [(&ok_name, true), (&fail_name, false)] {
            let arm = if ok { "Ok" } else { "Fail" };
            let vline = plan
                .p
                .produces_variants
                .iter()
                .find(|(l, _, _)| l.contains(if ok { "(ok" } else { "(fail" }))
                .map(|(_, _, l)| *l)
                .unwrap_or(plan.p.line);
            let _ = writeln!(
                s,
                "/// Everything P{}'s {arm} arm produces (R17, SPEC.md §5 'Produces ({arm})',",
                plan.p.id
            );
            let _ = writeln!(
                s,
                "/// line {vline}): the arm's products plus every conserved participant — the"
            );
            let _ = writeln!(
                s,
                "/// person returns at the same budget in both arms (the time draw is adjacent,"
            );
            let _ = writeln!(
                s,
                "/// F-048), reusables return in both arms (R2), and the stated waste leaves in"
            );
            let _ = writeln!(s, "/// both arms.");
            let _ = writeln!(s, "///");
            let _ = writeln!(
                s,
                "/// A grouping (R1), not a sealed resource (F-046): public fields the handling"
            );
            let _ = writeln!(
                s,
                "/// arm destructures, buildable only from already-held resources. Deliberately"
            );
            let _ = writeln!(
                s,
                "/// **no `Debug`** (F-047): `.unwrap()`/`.expect()` on the process `Result` do"
            );
            let _ = writeln!(s, "/// not compile — the flow must `match` both arms.");
            let _ = writeln!(
                s,
                "#[must_use = \"{name} bundles conserved outputs: every field must be accounted for\"]"
            );
            let generics = if plan.person { "<const B: u64>" } else { "" };
            let _ = writeln!(s, "pub struct {name}{generics} {{");
            for (fname, fty) in bundle_fields(plan, ok) {
                let _ = writeln!(s, "    pub {fname}: {fty},");
            }
            let _ = writeln!(s, "}}");
            let _ = writeln!(s);
        }
    }
}

fn emit_supplier(g: &Generator, s: &mut String, sp: &SupplierPlan) {
    let (ident, item) = (&sp.ident, &sp.item);
    let _ = writeln!(
        s,
        r#"/// Generated from SPEC.md §4 line {line}: a supplier of {item} at the system
/// boundary (R12), capacity {cap}. Contents are a type-level list of real
/// values; the count is the list's length.
#[must_use = "{ident} is a boundary resource: pass it on like any other resource"]
pub struct {ident}<Items>(Items);

/// The exhausted supplier: a distinct resource that must itself be accounted for (R12).
pub type Empty{ident} = {ident}<Nil>;

/// `Supplier` only for a non-empty list (R12): supplying from an empty
/// {ident} is a compile error with model-core's `on_unimplemented` message.
impl<H, T> Supplier for {ident}<Cons<H, T>> {{
    type Item = H;
    type Next = {ident}<T>;
    fn supply(self) -> (H, {ident}<T>) {{
        let Cons(head, tail) = self.0;
        (head, {ident}(tail))
    }}
}}

impl<Items: Len> {ident}<Items> {{
    /// How many items the supplier holds — the length of its contents list (R7, R12).
    pub const COUNT: u64 = Items::LEN;
}}
"#,
        line = sp.line,
        cap = sp.capacity,
    );
    let _ = g;
}

fn emit_bounded_consumer(s: &mut String, c: &BoundedConsumer) {
    let (ident, item) = (&c.ident, &c.item);
    let _ = writeln!(
        s,
        r#"/// Generated from SPEC.md §4 line {line}: a bounded consumer of {item}
/// (capacity {cap}) that KEEPS what it consumes (R12; the decreasing space
/// parameter is the F-034 hard rule).
#[must_use = "{ident} is a resource (R12): pass it on, or return it to the boundary"]
pub struct {ident}<Space, Contents = Nil> {{
    contents: Contents,
    _space: PhantomData<Space>,
}}

/// The full state: a distinct resource that must itself be accounted for (R12).
pub type Full{ident}<Contents> = {ident}<Zero, Contents>;

/// `Consumer` only while space remains (R12, F-034): space goes down by one
/// and the real consumed object is kept at the front of the contents list.
impl<S, C> Consumer<{item}> for {ident}<Succ<S>, C> {{
    type Next = {ident}<S, Cons<{item}, C>>;
    fn consume(self, item: {item}) -> Self::Next {{
        {ident} {{
            contents: Cons(item, self.contents),
            _space: PhantomData,
        }}
    }}
}}

impl<Space, Contents: Len> {ident}<Space, Contents> {{
    /// How many items the consumer holds — the length of its contents list (R7, R12).
    pub const HELD: u64 = Contents::LEN;
}}
"#,
        line = c.line,
        cap = c.capacity,
    );
}

fn emit_sink(s: &mut String, sink: &Sink) {
    let ident = &sink.ident;
    let _ = writeln!(
        s,
        r#"/// Generated from SPEC.md §4 line {line}: an unbounded boundary sink
/// (`Next = Self`, R15, F-029 — legal only at the system boundary; it
/// necessarily discards what it consumes).
///
/// Placeholder: {status}.
#[must_use = "{ident} is a boundary resource: pass it on like any other resource"]
pub struct {ident} {{
    _seal: (),
}}
"#,
        line = sink.line,
        status = clean(&sink.placeholder_note),
    );
    for (item, n) in &sink.consumes {
        let (gen_params, args) = if *n == 0 {
            (String::new(), String::new())
        } else {
            let names: Vec<String> = (0..*n).map(|i| format!("C{i}")).collect();
            (
                format!(
                    "<{}>",
                    names
                        .iter()
                        .map(|n| format!("const {n}: u64"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                format!("<{}>", names.join(", ")),
            )
        };
        let _ = writeln!(
            s,
            r#"/// The sink accepts {item} at any magnitude; `Next = Self` (R15).
impl{gen_params} Consumer<{item}{args}> for {ident} {{
    type Next = {ident};
    fn consume(self, item: {item}{args}) -> {ident} {{
        item.defuse(); // sanctioned consumer role (F-008); unbounded sink discards (F-029)
        self
    }}
}}
"#
        );
    }
}

fn emit_boundary_mod(g: &mut Generator, s: &mut String) {
    let _ = writeln!(
        s,
        "/// The creation boundary (R12): the only production code where boundary\n/// objects and supplied contents come into existence (SPEC.md §4).\npub mod boundary {{"
    );
    // imports
    let mut super_names: Vec<String> = Vec::new();
    for e in g.lexicon.iter() {
        if matches!(e.role, Role::Reusable) && !e.alias_only {
            super_names.push(e.ident.clone());
        }
    }
    for d in &g.draws {
        super_names.push(d.obj_ident.clone());
        if !super_names.contains(&d.item) {
            super_names.push(d.item.clone());
        }
    }
    for b in &g.bags {
        super_names.push(b.obj_ident.clone());
        if !super_names.contains(&b.item) {
            super_names.push(b.item.clone());
        }
    }
    for so in &g.start_objects {
        super_names.push(so.ident.clone());
    }
    for r in &g.rest_exits {
        super_names.push(r.ident.clone());
    }
    for sp in &g.suppliers {
        super_names.push(sp.ident.clone());
        if !super_names.contains(&sp.item) {
            super_names.push(sp.item.clone());
        }
    }
    for c in &g.consumers {
        super_names.push(c.ident.clone());
    }
    for sk in &g.sinks {
        super_names.push(sk.ident.clone());
    }
    if !g.consumers.is_empty() {
        super_names.push("PhantomData".into());
    }
    super_names.sort();
    super_names.dedup();
    let _ = writeln!(s, "    use super::{{{}}};", super_names.join(", "));
    if !g.suppliers.is_empty() || !g.consumers.is_empty() {
        let _ = writeln!(s, "    use model_core::list::{{Cons, Nil}};");
    }
    if !g.suppliers.is_empty() {
        let _ = writeln!(s, "    use model_core::nat::{{Succ, Zero}};");
    }
    let _ = writeln!(s);
    // outcome tokens: their constructors and exits are generated by
    // `model_core::outcome_token!` at the family level; re-exported so flows
    // find every boundary entry point here (R12)
    let outcome_fns: Vec<String> = {
        let mut v: Vec<String> = g
            .lexicon
            .iter()
            .filter(|e| matches!(e.role, Role::Outcome) && !e.alias_only)
            .flat_map(|e| {
                let sn = snake(&e.ident);
                vec![
                    format!("{sn}_will_fail"),
                    format!("{sn}_will_succeed"),
                    format!("return_{sn}"),
                ]
            })
            .collect();
        v.sort();
        v
    };
    if !outcome_fns.is_empty() {
        let _ = writeln!(
            s,
            "    /// The outcome tokens' boundary constructors and exits (R17), generated by\n    /// `model_core::outcome_token!` at the family level and re-exported here so the\n    /// creation boundary stays the single entry point (R12).\n    pub use super::{{{}}};\n",
            outcome_fns.join(", ")
        );
    }
    // reusable constructors
    let reusables: Vec<String> = g
        .lexicon
        .iter()
        .filter(|e| matches!(e.role, Role::Reusable) && !e.alias_only)
        .map(|e| e.ident.clone())
        .collect();
    for r in reusables {
        let _ = writeln!(
            s,
            "    /// {r} enters the model at flow start (R12).\n    ///\n    /// Placeholder: setup at flow start (SPEC.md §4).\n    pub fn new_{sn}() -> {r} {{\n        {r}::mint()\n    }}\n",
            sn = snake(&r)
        );
    }
    // stateful start objects (a §3 Container state entering at flow start,
    // e.g. the clean tins)
    let start_objects = g.start_objects.clone();
    for so in &start_objects {
        let ty = if so.consts.is_empty() {
            so.ident.clone()
        } else {
            format!(
                "{}<{}>",
                so.ident,
                so.consts.iter().map(|c| lit(*c)).collect::<Vec<_>>().join(", ")
            )
        };
        let _ = writeln!(
            s,
            "    /// {ident} enters the model at flow start, at its §3-declared magnitudes\n    /// (R12; SPEC.md §4 line {line}).\n    ///\n    /// Placeholder: setup at flow start (SPEC.md §4).\n    pub fn new_{sn}() -> {ty} {{\n        {ident}::mint()\n    }}\n",
            ident = so.ident,
            sn = snake(&so.ident),
            line = so.line
        );
    }
    // draw sources
    let draws = g.draws.clone();
    for d in &draws {
        let _ = writeln!(
            s,
            "    /// {obj} enters the model (R12).\n    ///\n    /// Placeholder: unbounded boundary source (SPEC.md §4 line {line}).\n    pub fn new_{sn}() -> {obj} {{\n        {obj}::mint()\n    }}\n",
            obj = d.obj_ident,
            sn = snake(&d.obj_ident),
            line = d.line
        );
        let _ = writeln!(
            s,
            "    /// Draws `TAKE` of {item} from the boundary (R15: an unbounded source of\n    /// continuous material is a draw-style boundary process, never a `Supplier`\n    /// impl, F-028). The source object is returned (R2).\n    ///\n    /// Placeholder: unbounded boundary source (SPEC.md §4 line {line}).\n    pub fn {fname}<const TAKE: u64>(source: {obj}) -> ({item}<TAKE>, {obj}) {{\n        ({item}::mint(), source)\n    }}\n",
            item = d.item,
            obj = d.obj_ident,
            fname = d.fn_name,
            line = d.line
        );
    }
    // bounded continuous sources with remainder (R15 draw processes over the
    // §4 pantry containers)
    let bags = g.bags.clone();
    for b in &bags {
        let _ = writeln!(
            s,
            "    /// The {obj} enters the model full (R12): a bounded continuous source with\n    /// remainder, capacity {cap} (SPEC.md §4 line {line}).\n    ///\n    /// Placeholder: vendor/stock not modelled (SPEC.md §4).\n    pub fn new_{sn}() -> {obj}<{cap}> {{\n        {obj}::mint()\n    }}\n",
            obj = b.obj_ident,
            sn = snake(&b.obj_ident),
            cap = lit(b.capacity),
            line = b.line
        );
        let _ = writeln!(
            s,
            "    model_core::draw_process! {{\n        /// Draws `TAKE` of {item} from the {obj} (R15), leaving `LEFT` of `FULL`\n        /// (SPEC.md §4 line {line}); the remainder is caller-stated and checked at\n        /// compile time (F-022, F-030).\n        pub fn {fname}: {obj} => {item},\n        assert = \"conservation violated in {fname} (R15): TAKE + LEFT must equal FULL — is the draw larger than the container's remaining contents?\"\n    }}\n",
            item = b.item,
            obj = b.obj_ident,
            fname = b.fn_name,
            line = b.line
        );
    }
    // supplier fill machinery
    let suppliers = g.suppliers.clone();
    for sp in &suppliers {
        let (ident, item) = (&sp.ident, &sp.item);
        let _ = writeln!(
            s,
            r#"    /// Maps a type-level count to the list type of that many {item}s.
    pub trait Replicate {{
        /// The list type holding `Self`-many items.
        type List;
    }}
    impl Replicate for Zero {{
        type List = Nil;
    }}
    impl<N: Replicate> Replicate for Succ<N> {{
        type List = Cons<{item}, N::List>;
    }}

    /// Shorthand for [`Replicate::List`].
    pub type ItemsOf<N> = <N as Replicate>::List;

    /// Builds a list of real items. Private: the only place {item}s come
    /// into existence (R1), reachable only through [`{fill}`].
    trait Fill {{
        fn fill() -> Self;
    }}
    impl Fill for Nil {{
        fn fill() -> Nil {{
            Nil
        }}
    }}
    impl<T: Fill> Fill for Cons<{item}, T> {{
        fn fill() -> Self {{
            Cons({item}::mint(), T::fill())
        }}
    }}

    /// Public-in-signature but unimplementable-outside (sealed-trait pattern, F-026).
    pub trait FillSealed: sealed::Sealed {{
        #[doc(hidden)]
        fn fill_sealed() -> Self;
    }}
    impl<L: Fill + sealed::Sealed> FillSealed for L {{
        fn fill_sealed() -> Self {{
            L::fill()
        }}
    }}
    mod sealed {{
        use super::super::{item};
        use model_core::list::{{Cons, Nil}};
        pub trait Sealed {{}}
        impl Sealed for Nil {{}}
        impl<T: Sealed> Sealed for Cons<{item}, T> {{}}
    }}

    /// The fill function (R12): a full supplier of `N` real items enters the
    /// model here (SPEC.md §4 line {line}: capacity {cap}).
    ///
    /// Placeholder: vendor not modelled (SPEC.md §4).
    pub fn {fill}<N: Replicate>() -> {ident}<ItemsOf<N>>
    where
        ItemsOf<N>: FillSealed,
    {{
        {ident}(FillSealed::fill_sealed())
    }}
"#,
            fill = sp.fill_fn,
            line = sp.line,
            cap = sp.capacity,
        );
    }
    // bounded consumers + sinks constructors
    let consumers = g.consumers.clone();
    for c in &consumers {
        let _ = writeln!(
            s,
            "    /// An empty {ident} with `Space` slots enters the model (SPEC.md §4 line {line}).\n    /// Creating an empty consumer brings no resources into existence (R12).\n    pub fn new_{sn}<Space>() -> {ident}<Space, Nil> {{\n        {ident} {{\n            contents: Nil,\n            _space: PhantomData,\n        }}\n    }}\n",
            ident = c.ident,
            sn = snake(&c.ident),
            line = c.line
        );
    }
    let sinks = g.sinks.clone();
    for sk in &sinks {
        let _ = writeln!(
            s,
            "    /// {ident} enters the model (R12). An empty unbounded sink holds nothing.\n    pub fn new_{sn}() -> {ident} {{\n        {ident} {{ _seal: () }}\n    }}\n",
            ident = sk.ident,
            sn = snake(&sk.ident)
        );
    }
    // flow-end rests (R12 exits): one per declared §4 `(flow-end rest)` row
    // (A13) — never synthesized
    let mut rests = g.rest_exits.clone();
    rests.sort();
    for r in &rests {
        let (ident, n) = (&r.ident, r.n_consts);
        let (gen_params, args) = if n == 0 {
            (String::new(), String::new())
        } else {
            let names: Vec<String> = (0..n).map(|i| format!("C{i}")).collect();
            (
                format!(
                    "<{}>",
                    names
                        .iter()
                        .map(|n| format!("const {n}: u64"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                format!("<{}>", names.join(", ")),
            )
        };
        let _ = writeln!(
            s,
            "    /// Flow-end rest for {ident} (R12 exit): a §4 output row (SPEC.md §4 line\n    /// {line}, A13) declares it resting at the boundary at flow end, and an\n    /// integration test cannot hold a tripwired resource past its end — the\n    /// accounted counterpart of a constructor (the CS-2 `take_wallet_home` shape).\n    pub fn rest_{sn}{gen_params}(resting: {ident}{args}) {{\n        resting.defuse(); // the sanctioned boundary exit (R12, F-008)\n    }}\n",
            sn = snake(ident),
            line = r.line
        );
    }
    let _ = writeln!(s, "}}");
}

fn structural_only(b: &Balance) -> bool {
    match b.mark {
        BalanceMark::Structural => true,
        BalanceMark::Assert => false,
        // lenient-only guess (strict mode errors on Unmarked before emission):
        // identical single terms ⇒ structural
        BalanceMark::Unmarked => b.lhs.len() == 1 && b.rhs.len() == 1 && b.lhs == b.rhs,
    }
}

/// Consumes one input inside a process body: tripwired resources defuse (the
/// sanctioned forget, F-008); untripwired consumables have no `defuse`, so
/// they are consumed by destructuring (F-032, the CS-2 `DryPatch` shape).
fn consume_stmt(g: &Generator, s: &mut String, ident: &str, var: &str) {
    if is_untripwired(g, ident) {
        let _ = writeln!(
            s,
            "        let {ident} {{ _seal: _ }} = {var}; // conserving transform (untripwired, F-040): its magnitudes continue in the outputs (checked by the asserts above)"
        );
    } else {
        let _ = writeln!(
            s,
            "        {var}.defuse(); // conserving transform: its magnitudes continue in the outputs (checked by the asserts above)"
        );
    }
}

fn emit_process(g: &mut Generator, s: &mut String, plan: &ProcPlan) {
    let sig = build_sig(g, plan);
    let p = &plan.p;
    let fname = &plan.fn_name;
    let _ = writeln!(
        s,
        "    /// P{} — {} (SPEC.md §5 P{}, line {}).",
        p.id,
        clean(&p.title),
        p.id,
        p.line
    );
    if let Some(ms) = p.draws_ms {
        let _ = writeln!(
            s,
            "    /// The person's {} ms is drawn by the **adjacent** `draw_time` in the flow,\n    /// recorded to the History under this process's name (F-048 — the template's\n    /// stated convention); the process takes and returns the person unchanged.",
            lit(ms).replace('_', "\u{202f}")
        );
    }
    if p.no_person {
        let _ = writeln!(
            s,
            "    /// **No person** (SPEC.md Actor line): this is what creates the §6 ordering freedom."
        );
    }
    let satisfies: Vec<String> = p
        .satisfies
        .iter()
        .map(|sid| {
            let impl_id = g
                .spec
                .requirements
                .iter()
                .find(|r| r.spec_id == *sid)
                .map(|r| r.impl_id)
                .unwrap_or(*sid);
            format!("REQ-{impl_id:03}")
        })
        .collect();
    if !satisfies.is_empty() {
        let _ = writeln!(s, "    ///");
        let _ = writeln!(s, "    /// Satisfies: {}", satisfies.join(", "));
    }
    // holes attached to this process
    if !satisfies.is_empty() {
        s.push_str(&hole_block(
            g,
            "    ",
            &format!("{fname}: requirement bounds (R10 style A) are not derivable from the spec"),
            &format!(
                "SPEC.md §5 P{} says this process satisfies {}, but the generated signature\nuses concrete types: the requirement-trait bounds, the characteristic consts\nthey carry, and the REQ-phrased wrong-resource errors (F-044) are all\nimplementation design the spec does not determine. Wrong-resource errors here\nare plain E0308 type mismatches.",
                p.id,
                satisfies.join(", ")
            ),
        ));
    }
    // The strong (consumer-parameter) waste routing is implementation design
    // the scaffold does not build: it emits the weak reading and says so.
    let strong_stated = p
        .waste_routing
        .as_ref()
        .map(|r| r.to_lowercase().contains("consumer parameter"))
        .unwrap_or(false);
    let weak_emitted = plan.waste.iter().any(|w| w.duplicate_of_produce)
        || (strong_stated && !plan.waste.is_empty());
    if weak_emitted && plan.bin_disposal.is_none() && (strong_stated || p.waste_routing.is_none())
    {
        s.push_str(&hole_block(
            g,
            "    ",
            &format!("{fname}: the stated consumer-parameter waste routing is not scaffolded"),
            &format!(
                "SPEC.md §5 P{} routes waste as a consumer parameter (the STRONG reading:\nthe process takes its consumer as a requirement-bounded parameter and feeds\nit internally, so the waste never exists loose). The scaffold emits the WEAK\nreading — loose outputs routed by the flow — because the bounded-consumer\nparameter design (which bound, fed where) is implementation judgement.",
                p.id
            ),
        ));
    }
    if plan.supplier_consumed {
        if let Some((_, item)) = &plan.supply_n {
            if let Some(sp) = g.suppliers.iter().find(|s| &s.item == item).cloned() {
                s.push_str(&hole_block(
                    g,
                    "    ",
                    &format!("{fname}: the exhausted supplier continues as the declared empty state by generator convention"),
                    &format!(
                        "The Consumes list names the {} that §4 also names as this process's\ndiscrete supplier. The scaffold reads them as ONE object: the supplier\nparameter IS the container (no second input parameter), `Rest` is pinned to\nEmpty{} so exhaustion is total, and the exhausted shell is destructured so the\n§3 empty state can be minted as the declared output. Whether the empty\ncontainer is the supplier's remainder or a separate object of its own mass is\nthe author's call — say so in the specification.",
                        sp.ident, sp.ident
                    ),
                ));
            }
        }
    }
    if fallible(plan) {
        let (ok_name, fail_name) = bundle_names(plan);
        s.push_str(&hole_block(
            g,
            "    ",
            &format!("{fname}: R17 refinements beyond the scaffolded dual-arm shape are not derivable"),
            &format!(
                "The scaffold realises the outcome token in a match and returns\nResult<{ok_name}, {fail_name}> with every conserved participant carried in both\narms (the CS-2 patch_tube shape). Not derivable from the spec: per-arm\ngeneralized const parameters with independent per-arm asserts (F-045),\nrequirement bounds on the arms, outcome calibration (the boundary constructors\nstay placeholders), and the flow-level #[must_use] outcome grouping over the\nstated outcome combinations (F-050) — the generated flows inject success\ntokens only."
            ),
        ));
    } else if !p.produces_variants.is_empty() {
        s.push_str(&hole_block(
            g,
            "    ",
            &format!("{fname}: the R17 dual-arm shape could not be scaffolded"),
            "This process states Produces (Ok)/(Fail) variants, but the scaffoldable\ndual-arm shape needs both arms non-empty, exactly one consumed outcome token,\nand no SupplyN draw in the same process. The variant items are name-checked\nonly; the dual-arm Result shape is left to the implementer.",
        ));
    }
    let mixed_state = plan.consumes.iter().any(|c| {
        held_of(g, &c.ident).is_some() || (is_container(g, &c.ident) && c.consts.len() > 1)
    });
    if mixed_state {
        s.push_str(&hole_block(
            g,
            "    ",
            &format!("{fname}: conserving extraction design is not derivable"),
            "The body below defuses consumed states and mints produced ones — conservation\nholds only via the stated asserts, not by construction. The real crates route\nthe magnitudes through sealed, permit-gated characteristic extractions so a\nfree-standing transform cannot vanish them; that machinery is pure\nimplementation design.",
        ));
    }
    // signature
    let generics = if sig.generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", sig.generics.join(", "))
    };
    let params = sig
        .params
        .iter()
        .map(|(n, t)| format!("{n}: {t}"))
        .collect::<Vec<_>>()
        .join(", ");
    let rets = sig
        .returns
        .iter()
        .map(|(_, t)| t.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let ret_clause = match sig.returns.len() {
        0 => String::new(),
        1 => format!(" -> {rets}"),
        _ => format!(" -> ({rets})"),
    };
    let _ = writeln!(s, "    pub fn {fname}{generics}({params}){ret_clause} {{");
    // asserts from balances
    for b in &p.balances {
        if b.symbolic || b.lhs.is_empty() || b.rhs.is_empty() {
            continue; // nothing arithmetical to restate
        }
        if structural_only(b) {
            let _ = writeln!(
                s,
                "        // {} balance '{}' (SPEC.md line {}): structural — carried by the shared magnitudes, no assert needed.",
                b.dimension,
                clean(&b.raw),
                b.line
            );
            continue;
        }
        let lhs = b.lhs.iter().map(|n| lit(*n)).collect::<Vec<_>>().join(" + ");
        let rhs = b.rhs.iter().map(|n| lit(*n)).collect::<Vec<_>>().join(" + ");
        let _ = writeln!(s, "        const {{");
        let _ = writeln!(s, "            assert!(");
        let _ = writeln!(s, "                {lhs} == {rhs},");
        let _ = writeln!(
            s,
            "                \"{} conservation violated in {} (SPEC.md §5 P{} line {}): {}\"",
            b.dimension,
            fname,
            p.id,
            b.line,
            clean(&b.raw)
        );
        let _ = writeln!(s, "            )");
        let _ = writeln!(s, "        }};");
    }
    // body
    if let Some(d) = &plan.bin_disposal {
        let bin_var = snake(&d.bin_ident);
        let sink_var = snake(&d.sink_ident);
        let mut pat = "Nil".to_string();
        for i in (1..=d.count).rev() {
            pat = format!("Cons(i{i}, {pat})");
        }
        let _ = writeln!(
            s,
            "        let {ident} {{\n            contents: {pat},\n            _space: PhantomData,\n        }} = {bin_var};",
            ident = d.bin_ident
        );
        for i in 1..=d.count {
            let _ = writeln!(
                s,
                "        i{i}.defuse(); // sealed disposal path (F-039): the kept mass continues as {} below",
                d.waste_ident
            );
        }
        let _ = writeln!(
            s,
            "        let {sink_var} = send_to({sink_var}, {}::<{}>::mint());",
            d.waste_ident,
            lit(d.waste_mass)
        );
        let mut ret_items = Vec::new();
        if plan.person {
            ret_items.push("person".to_string());
        }
        ret_items.push(format!(
            "{} {{\n                contents: Nil,\n                _space: PhantomData,\n            }}",
            d.bin_ident
        ));
        ret_items.push(sink_var.clone());
        let _ = writeln!(s, "        (");
        for r in &ret_items {
            let _ = writeln!(s, "            {r},");
        }
        let _ = writeln!(s, "        )");
        let _ = writeln!(s, "    }}");
        let _ = writeln!(s);
        return;
    }
    // supply destructure
    if let Some((n, item)) = &plan.supply_n {
        if let Some(sp) = g.suppliers.iter().find(|s| &s.item == item).cloned() {
            let mut pat = "Nil".to_string();
            for i in (1..=*n).rev() {
                pat = format!("Cons(i{i}, {pat})");
            }
            let _ = writeln!(
                s,
                "        let ({pat}, rest) = {}.supply_n(); // one at a time via SupplyN (R12, F-014)",
                snake(&sp.ident)
            );
            // supplied items not kept by a held output are consumed here
            let held_used = sig.returns.iter().any(|(_, t)| {
                let base = t.split('<').next().unwrap_or(t).trim();
                held_of(g, base)
                    .map(|(_, held_item)| &held_item == item)
                    .unwrap_or(false)
            });
            if !held_used {
                for i in 1..=*n {
                    consume_stmt(g, s, item, &format!("i{i}"));
                }
            }
            if plan.supplier_consumed {
                let _ = writeln!(
                    s,
                    "        let {}(Nil) = rest; // the exhausted supplier's shell (Rest = Empty{}): its mass continues as the empty-state output",
                    sp.ident, sp.ident
                );
            }
        }
    }
    // consumed values
    for c in &plan.consumes {
        if plan
            .supply_n
            .as_ref()
            .map(|(_, item)| item == &c.ident)
            .unwrap_or(false)
        {
            continue; // consumed via the SupplyN destructure above
        }
        if fallible(plan) && plan.outcome.as_deref() == Some(&c.ident) {
            continue; // the outcome token is realised by the match below
        }
        if c.count > 1 {
            for i in 1..=c.count {
                consume_stmt(g, s, &c.ident, &format!("{}_{i}", snake(&c.ident)));
            }
        } else {
            consume_stmt(g, s, &c.ident, &snake(&c.ident));
        }
    }
    for r in &plan.reusable_params {
        // a vessel that continues as a produced state is destructured; a
        // passthrough reusable is returned as-is
        if !plan.reusable_returns.contains(r) {
            let _ = writeln!(
                s,
                "        let {r} {{ _seal: () }} = {var}; // the vessel continues as the produced state",
                var = snake(r)
            );
        }
    }
    // R17 dual-arm body: the outcome token is realised here and nowhere else;
    // both arms are constructed, so neither can be left unwritten (F-042)
    if fallible(plan) {
        let (ok_name, fail_name) = bundle_names(plan);
        let outcome = plan.outcome.clone().unwrap_or_default();
        let kind = format!("{outcome}Kind");
        let ovar = snake(&outcome);
        let _ = writeln!(s, "        match {ovar}.consume_kind() {{");
        for (variant, name, wrap, ok) in [
            ("Success", &ok_name, "Ok", true),
            ("Failure", &fail_name, "Err", false),
        ] {
            let _ = writeln!(s, "            {kind}::{variant} => {wrap}({name} {{");
            for (field, fty) in bundle_fields(plan, ok) {
                let base = fty.split('<').next().unwrap_or(&fty).trim().to_string();
                let expr = if field == "person" {
                    "person".to_string()
                } else if plan.reusable_returns.contains(&base)
                    && plan.reusable_params.contains(&base)
                {
                    // passthrough reusable: moved in, returned in both arms (R2)
                    field.clone()
                } else if plan.reusable_returns.contains(&base) {
                    // the consumed state's vessel returns to its bare state
                    format!("{base}::mint()")
                } else {
                    format!("{base}::mint()")
                };
                if expr == field {
                    let _ = writeln!(s, "                {field},");
                } else {
                    let _ = writeln!(s, "                {field}: {expr},");
                }
            }
            let _ = writeln!(s, "            }}),");
        }
        let _ = writeln!(s, "        }}");
        let _ = writeln!(s, "    }}");
        let _ = writeln!(s);
        return;
    }
    // return tuple
    let mut ret_exprs: Vec<String> = Vec::new();
    for (name, t) in &sig.returns {
        if name == "person" {
            ret_exprs.push("person".into());
        } else if t == "S::Rest" {
            ret_exprs.push("rest".into());
        } else if plan.reusable_returns.contains(t) && plan.reusable_params.contains(t) {
            // passthrough reusable: moved in, returned unchanged (R2)
            ret_exprs.push(name.clone());
        } else if plan.reusable_returns.contains(t) {
            // the consumed state's vessel returns to its bare state
            ret_exprs.push(format!("{t}::mint()"));
        } else {
            // produced / waste item: mint it
            let base = t.split('<').next().unwrap_or(t);
            if let Some((n, item)) = held_of(g, base) {
                let args = (1..=n).map(|i| format!("i{i}")).collect::<Vec<_>>().join(", ");
                ret_exprs.push(format!(
                    "{base}::mint(({args})) /* holds the {n} real {item}s */"
                ));
            } else {
                ret_exprs.push(format!("{base}::mint()"));
            }
        }
    }
    match ret_exprs.len() {
        0 => {}
        1 => {
            let _ = writeln!(s, "        {}", ret_exprs[0]);
        }
        _ => {
            let _ = writeln!(s, "        (");
            for r in &ret_exprs {
                let _ = writeln!(s, "            {r},");
            }
            let _ = writeln!(s, "        )");
        }
    }
    let _ = writeln!(s, "    }}");
    let _ = writeln!(s);
}

fn emit_resources(g: &mut Generator) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "//! GENERATED sealed resource family (R1), boundary (R12) and processes\n//! (SPEC.md §3, §4, §5). Layout per F-006/F-031: types here, with `boundary`\n//! and `processes` child modules."
    );
    let _ = writeln!(s);
    if !g.consumers.is_empty() {
        let _ = writeln!(s, "use core::marker::PhantomData;");
    }
    if !g.suppliers.is_empty() {
        let _ = writeln!(s, "use model_core::boundary::{{Consumer, Supplier}};");
        let _ = writeln!(s, "use model_core::list::{{Cons, Len, Nil}};");
    } else {
        let _ = writeln!(s, "use model_core::boundary::Consumer;");
    }
    if !g.consumers.is_empty() {
        let _ = writeln!(s, "use model_core::nat::{{Succ, Zero}};");
    }
    if g.plans.iter().any(|p| fallible(p) && p.person) {
        let _ = writeln!(s, "use model_core::common::Person;");
    }
    let _ = writeln!(s);
    s.push_str(&hole_block(
        g,
        "",
        "magnitudes are generated as the spec's literal numbers, not generic const parameters",
        "The spec states one worked instance. The generator emits those literals in\nsignatures and types; the real crates generalize most processes over const\nparameters, with the spec's numbers appearing only at call sites. Where to\ngeneralize is a design judgement the spec does not state.",
    ));
    let _ = writeln!(s);
    emit_resource_types(g, &mut s);
    emit_bundles(g, &mut s);
    let suppliers = g.suppliers.clone();
    for sp in &suppliers {
        emit_supplier(g, &mut s, sp);
    }
    let consumers = g.consumers.clone();
    for c in &consumers {
        emit_bounded_consumer(&mut s, c);
    }
    let sinks = g.sinks.clone();
    for sk in &sinks {
        emit_sink(&mut s, sk);
    }
    emit_boundary_mod(g, &mut s);
    let _ = writeln!(s);
    // processes module
    let _ = writeln!(
        s,
        "/// GENERATED processes (SPEC.md §5): by-value conserving transformations\n/// (R1, R2), living inside the resource family's privacy boundary (F-031)."
    );
    let _ = writeln!(s, "pub mod processes {{");
    // imports: every non-common lexicon ident used in plans + helpers
    let mut names: Vec<String> = Vec::new();
    let plans = g.plans.clone();
    for plan in &plans {
        let sig = build_sig(g, plan);
        for (_, t) in sig.params.iter().chain(sig.returns.iter()) {
            let base = t.split('<').next().unwrap_or(t).trim().to_string();
            if base != "S" && base != "S::Rest" && base != "Person" && base != "Result"
                && !base.is_empty()
            {
                names.push(base);
            }
        }
        if let Some(d) = &plan.bin_disposal {
            names.push(d.waste_ident.clone());
            names.push(d.item.clone());
        }
        if let Some((_, item)) = &plan.supply_n {
            names.push(item.clone());
            if plan.supplier_consumed {
                if let Some(sp) = g.suppliers.iter().find(|s| &s.item == item) {
                    // the Rest bound and the exhausted-shell destructure
                    names.push(format!("Empty{}", sp.ident));
                    names.push(sp.ident.clone());
                }
            }
        }
        if fallible(plan) {
            let (ok_name, fail_name) = bundle_names(plan);
            names.push(ok_name);
            names.push(fail_name);
            if let Some(o) = &plan.outcome {
                names.push(format!("{o}Kind"));
            }
            // the arms' minted products and waste are not in the signature
            for (_, t) in bundle_fields(plan, true)
                .iter()
                .chain(bundle_fields(plan, false).iter())
            {
                let base = t.split('<').next().unwrap_or(t).trim().to_string();
                if base != "Person" && !base.is_empty() {
                    names.push(base);
                }
            }
        }
        for pr in &plan.produces {
            if let Some((_, held_item)) = held_of(g, &pr.ident) {
                names.push(held_item);
            }
        }
    }
    let any_disposal = plans.iter().any(|p| p.bin_disposal.is_some());
    let any_supply = plans.iter().any(|p| p.supply_n.is_some());
    if any_disposal {
        names.push("PhantomData".into());
    }
    names.sort();
    names.dedup();
    let _ = writeln!(s, "    use super::{{{}}};", names.join(", "));
    match (any_supply, any_disposal) {
        (true, true) => {
            let _ = writeln!(s, "    use model_core::boundary::{{SupplyN, send_to}};");
        }
        (true, false) => {
            let _ = writeln!(s, "    use model_core::boundary::SupplyN;");
        }
        (false, true) => {
            let _ = writeln!(s, "    use model_core::boundary::send_to;");
        }
        (false, false) => {}
    }
    if plans.iter().any(|p| p.person) {
        let _ = writeln!(s, "    use model_core::common::Person;");
    }
    if any_supply || any_disposal {
        let _ = writeln!(s, "    use model_core::list::{{Cons, Nil}};");
    }
    // nat aliases used
    let mut aliases: Vec<String> = Vec::new();
    for plan in &plans {
        if let Some((n, _)) = &plan.supply_n {
            aliases.push(format!("N{n}"));
        }
        if let Some(d) = &plan.bin_disposal {
            aliases.push(format!("N{}", d.cap));
            aliases.push(format!("N{}", d.cap - d.count));
        }
    }
    aliases.sort();
    aliases.dedup();
    if !aliases.is_empty() {
        let _ = writeln!(s, "    use model_core::nat::aliases::{{{}}};", aliases.join(", "));
    }
    let _ = writeln!(s);
    for plan in &plans {
        emit_process(g, &mut s, plan);
    }
    let _ = writeln!(s, "}}");
    s
}

// ── flows ────────────────────────────────────────────────────────────────

fn emit_flows(g: &mut Generator, crate_ident: &str) -> String {
    // The test bodies are composed first (into `body`), then prefixed with
    // the import list; the flow-end `rest_*` exits both use are declared by
    // the §4 `(flow-end rest)` rows (A13), never discovered here.
    let mut body = String::new();
    {
        let s = &mut body;
        let orders = g.spec.flows.orders.clone();
        let flows_line = g.spec.flows.line;
        let mut names_seen = Vec::new();
        for (oi, order) in orders.iter().enumerate() {
            let letter = (b'a' + oi as u8) as char;
            let name = format!("flow_order_{letter}_type_checks_and_accounts_for_everything");
            if names_seen.contains(&name) {
                continue;
            }
            names_seen.push(name.clone());
            let order_str = order
                .iter()
                .map(|p| format!("P{p}"))
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(
                s,
                "/// GENERATED flow order ({letter}) from SPEC.md §6 (line {flows_line}): {order_str}."
            );
            let verifies: Vec<String> = {
                let mut v: Vec<u32> = g
                    .spec
                    .processes
                    .iter()
                    .flat_map(|p| p.satisfies.iter())
                    .map(|sid| {
                        g.spec
                            .requirements
                            .iter()
                            .find(|r| r.spec_id == *sid)
                            .map(|r| r.impl_id)
                            .unwrap_or(*sid)
                    })
                    .collect();
                v.sort();
                v.dedup();
                v.iter().map(|id| format!("REQ-{id:03}")).collect()
            };
            if !verifies.is_empty() {
                let _ = writeln!(s, "///");
                let _ = writeln!(s, "/// Verifies: {}", verifies.join(", "));
            }
            let _ = writeln!(s, "#[test]");
            let _ = writeln!(s, "fn {name}() {{");
            emit_flow_body(g, s, order);
            let _ = writeln!(s, "}}");
            let _ = writeln!(s);
        }
    }
    let mut s = String::new();
    let _ = writeln!(
        s,
        "//! GENERATED flow skeletons (SPEC.md §6): each agreed valid order composed\n//! as an integration test, with the person's budget drawn by adjacent\n//! `draw_time` steps recorded to a single History (R16, F-048)."
    );
    let _ = writeln!(s);
    let _ = writeln!(s, "#![forbid(unsafe_code)]");
    let _ = writeln!(s, "#![deny(unused_must_use)]");
    let _ = writeln!(s, "#![deny(let_underscore_drop)]");
    let _ = writeln!(s, "#![recursion_limit = \"2048\"]");
    let _ = writeln!(s);
    // imports
    let mut boundary_fns: Vec<String> = Vec::new();
    for e in g
        .lexicon
        .iter()
        .filter(|e| matches!(e.role, Role::Reusable) && !e.alias_only)
    {
        boundary_fns.push(format!("new_{}", snake(&e.ident)));
    }
    for d in &g.draws {
        boundary_fns.push(format!("new_{}", snake(&d.obj_ident)));
        boundary_fns.push(d.fn_name.clone());
    }
    for b in &g.bags {
        boundary_fns.push(format!("new_{}", snake(&b.obj_ident)));
        boundary_fns.push(b.fn_name.clone());
    }
    for so in &g.start_objects {
        boundary_fns.push(format!("new_{}", snake(&so.ident)));
    }
    for e in g
        .lexicon
        .iter()
        .filter(|e| matches!(e.role, Role::Outcome) && !e.alias_only)
    {
        // the scaffolded flows inject success tokens (placeholder; see the
        // process's SPEC-HOLE for the outcome-combination design)
        boundary_fns.push(format!("{}_will_succeed", snake(&e.ident)));
    }
    for sp in &g.suppliers {
        boundary_fns.push(sp.fill_fn.clone());
    }
    for c in &g.consumers {
        boundary_fns.push(format!("new_{}", snake(&c.ident)));
    }
    for sk in &g.sinks {
        boundary_fns.push(format!("new_{}", snake(&sk.ident)));
    }
    for r in &g.rest_exits {
        boundary_fns.push(format!("rest_{}", snake(&r.ident)));
    }
    boundary_fns.sort();
    boundary_fns.dedup();
    let proc_fns: Vec<String> = g.plans.iter().map(|p| p.fn_name.clone()).collect();
    let _ = writeln!(
        s,
        "use {crate_ident}::resources::boundary::{{{}}};",
        boundary_fns.join(", ")
    );
    let _ = writeln!(
        s,
        "use {crate_ident}::resources::processes::{{{}}};",
        proc_fns.join(", ")
    );
    let mut bundle_imports: Vec<String> = g
        .plans
        .iter()
        .filter(|p| fallible(p))
        .flat_map(|p| {
            let (ok_name, fail_name) = bundle_names(p);
            vec![fail_name, ok_name]
        })
        .collect();
    bundle_imports.sort();
    bundle_imports.dedup();
    if !bundle_imports.is_empty() {
        let _ = writeln!(
            s,
            "use {crate_ident}::resources::{{{}}};",
            bundle_imports.join(", ")
        );
    }
    let _ = writeln!(s, "use model_core::boundary::send_to;");
    let _ = writeln!(s, "use model_core::common::Person;");
    let _ = writeln!(s, "use model_core::common::boundary::new_person;");
    let _ = writeln!(s, "use model_core::common::processes::draw_time;");
    let _ = writeln!(s, "use model_core::history::boundary::new_history;");
    let _ = writeln!(s, "use model_core::history::processes::record;");
    let mut aliases: Vec<String> = Vec::new();
    for c in &g.consumers {
        aliases.push(format!("N{}", c.capacity));
    }
    for sp in &g.suppliers {
        aliases.push(format!("N{}", sp.capacity));
    }
    aliases.sort();
    aliases.dedup();
    if !aliases.is_empty() {
        let _ = writeln!(s, "use model_core::nat::aliases::{{{}}};", aliases.join(", "));
    }
    let _ = writeln!(s);
    s.push_str(&body);
    s
}

fn emit_flow_body(g: &mut Generator, s: &mut String, order: &[u32]) {
    let _ = writeln!(s, "    let history = new_history();");
    let budget = g.person_budget;
    let _ = writeln!(s, "    let person = new_person::<{}>();", lit(budget));
    // sinks and bounded consumers up front
    for c in &g.consumers {
        let _ = writeln!(
            s,
            "    let {} = new_{}::<N{}>();",
            snake(&c.ident),
            snake(&c.ident),
            c.capacity
        );
    }
    for sk in &g.sinks {
        let _ = writeln!(s, "    let {} = new_{}();", snake(&sk.ident), snake(&sk.ident));
    }
    let mut remaining = budget;
    let mut live: Vec<String> = vec!["person".into()];
    for c in &g.consumers {
        live.push(snake(&c.ident));
    }
    for sk in &g.sinks {
        live.push(snake(&sk.ident));
    }
    // running remainder per bounded container source (the pantry rows)
    let mut bag_left: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    // each live binding's base type, for the flow-end rest accounting
    let mut var_ty: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    // a fresh binding name: never shadow a live resource (shadowing leaks it)
    let fresh = |live: &[String], name: &str| -> String {
        if !live.iter().any(|v| v == name) {
            return name.to_string();
        }
        let mut k = 2;
        loop {
            let cand = format!("{name}_{k}");
            if !live.iter().any(|v| v == &cand) {
                return cand;
            }
            k += 1;
        }
    };
    let plans = g.plans.clone();
    for pid in order {
        let Some(plan) = plans.iter().find(|p| p.p.id == *pid) else {
            continue;
        };
        let sig = build_sig(g, plan);
        let _ = writeln!(
            s,
            "    // P{} — {} (SPEC.md §5 line {}).",
            plan.p.id,
            clean(&plan.p.title),
            plan.p.line
        );
        // resolve each parameter to a live value, drawing/constructing
        // boundary objects where the spec provides a source
        let mut args: Vec<String> = Vec::new();
        for (pname, pty) in &sig.params {
            if live.contains(pname) {
                args.push(pname.clone());
                continue;
            }
            // a produced batch feeds repeated runs: take the next numbered
            // item still live (shaped_loaf → shaped_loaf_1, then _2)
            if let Some(v) = (1..=16u64)
                .map(|k| format!("{pname}_{k}"))
                .find(|v| live.contains(v))
            {
                args.push(v);
                continue;
            }
            let base = pty.split('<').next().unwrap_or(pty).trim();
            if let Some(d) = g.draws.iter().find(|d| d.item == base) {
                let src = snake(&d.obj_ident);
                if !live.contains(&src) {
                    let _ = writeln!(s, "    let {src} = new_{src}();");
                    live.push(src.clone());
                }
                let amount = plan
                    .consumes
                    .iter()
                    .find(|c| c.ident == base)
                    .and_then(|c| c.consts.first().copied())
                    .unwrap_or(0);
                let _ = writeln!(
                    s,
                    "    let ({pname}, {src}) = {}::<{}>({src});",
                    d.fn_name,
                    lit(amount)
                );
                live.push(pname.clone());
                var_ty.insert(pname.clone(), base.to_string());
            } else if let Some(b) = g.bags.iter().find(|b| b.item == base) {
                // bounded container source: draw with the caller-stated
                // remainder (F-030); the generator re-derives the running
                // balance the §6 account states
                let src = snake(&b.obj_ident);
                if !live.contains(&src) {
                    let _ = writeln!(s, "    let {src} = new_{src}();");
                    live.push(src.clone());
                    var_ty.insert(src.clone(), b.obj_ident.clone());
                    bag_left.insert(src.clone(), b.capacity);
                }
                let amount = plan
                    .consumes
                    .iter()
                    .find(|c| c.ident == base)
                    .and_then(|c| c.consts.first().copied())
                    .unwrap_or(0);
                let full = bag_left.get(&src).copied().unwrap_or(b.capacity);
                let left = full.saturating_sub(amount);
                bag_left.insert(src.clone(), left);
                let _ = writeln!(
                    s,
                    "    let ({pname}, {src}) = {}::<{}, {}, {}>({src});",
                    b.fn_name,
                    lit(amount),
                    lit(left),
                    lit(full)
                );
                live.push(pname.clone());
                var_ty.insert(pname.clone(), base.to_string());
            } else if pty == "S" {
                if let Some(sp) = g.suppliers.iter().find(|sp| snake(&sp.ident) == *pname) {
                    let _ = writeln!(s, "    let {pname} = {}::<N{}>();", sp.fill_fn, sp.capacity);
                    live.push(pname.clone());
                }
            } else if g
                .lexicon
                .iter()
                .any(|e| e.ident == base && !e.alias_only && matches!(e.role, Role::Reusable))
            {
                let _ = writeln!(s, "    let {pname} = new_{pname}();");
                live.push(pname.clone());
            } else if let Some(so) = g.start_objects.iter().find(|so| so.ident == base) {
                let _ = writeln!(s, "    let {pname} = new_{}();", snake(&so.ident));
                live.push(pname.clone());
                var_ty.insert(pname.clone(), base.to_string());
            } else if g
                .lexicon
                .iter()
                .any(|e| e.ident == base && !e.alias_only && matches!(e.role, Role::Outcome))
            {
                let _ = writeln!(s, "    let {pname} = {}_will_succeed();", snake(base));
                live.push(pname.clone());
            }
            args.push(pname.clone());
        }
        for a in &args {
            live.retain(|v| v != a);
        }
        let args = args.join(", ");
        if fallible(plan) {
            emit_flow_fallible_call(g, s, plan, &args, &mut live, &mut var_ty, &fresh);
        } else {
            // fresh output bindings: rebinding a live resource would shadow
            // (and so leak) it
            let mut out_names: Vec<String> = Vec::new();
            for (rn, rt) in &sig.returns {
                let name = fresh(&live, rn);
                out_names.push(name.clone());
                live.push(name.clone());
                let base = rt.split('<').next().unwrap_or(rt).trim().to_string();
                var_ty.insert(name, base);
            }
            let outs = out_names.join(", ");
            if sig.returns.len() == 1 {
                let _ = writeln!(s, "    let {outs} = {}({args});", plan.fn_name);
            } else {
                let _ = writeln!(s, "    let ({outs}) = {}({args});", plan.fn_name);
            }
            // route loose waste (and waste-duplicated produce) to its sink;
            // the generated names are the signature's return names
            let rename: std::collections::BTreeMap<&String, &String> = sig
                .returns
                .iter()
                .map(|(rn, _)| rn)
                .zip(out_names.iter())
                .collect();
            for w in &plan.waste {
                let Some(sink_var) = &w.sink_var else { continue };
                let Some(item) = &w.item else { continue };
                let mut names: Vec<String> = Vec::new();
                if w.duplicate_of_produce {
                    if let Some(pr) = plan.produces.iter().find(|pr| pr.ident == item.ident) {
                        if pr.count > 1 {
                            for i in 1..=pr.count {
                                names.push(format!("{}_{i}", snake(&pr.ident)));
                            }
                        } else {
                            names.push(snake(&pr.ident));
                        }
                    }
                } else if item.count > 1 {
                    for i in 1..=item.count {
                        names.push(format!("{}_{i}", snake(&item.ident)));
                    }
                } else {
                    names.push(snake(&item.ident));
                }
                for n in names {
                    let v = rename.get(&n).map(|v| (*v).clone()).unwrap_or(n);
                    let _ = writeln!(s, "    let {sink_var} = send_to({sink_var}, {v});");
                    live.retain(|x| x != &v);
                }
            }
        }
        // adjacent time draw (F-048)
        if let Some(ms) = plan.p.draws_ms {
            let left = remaining - ms;
            let _ = writeln!(
                s,
                "    let (labour, person) = draw_time::<{}, {}, {}>(person);",
                lit(ms),
                lit(left),
                lit(remaining)
            );
            let _ = writeln!(
                s,
                "    let history = record(history, \"{}\", labour);",
                plan.fn_name
            );
            remaining = left;
        }
    }
    // route remaining products to their §4 sinks (e.g. the pot of tea to the drinker)
    let _ = writeln!(s, "    // Everything accounted at flow end (SPEC.md §6).");
    for sk in &g.sinks {
        for (item, _) in &sk.consumes {
            let v = snake(item);
            if live.contains(&v) {
                let sv = snake(&sk.ident);
                let _ = writeln!(s, "    let {sv} = send_to({sv}, {v});");
                live.retain(|x| x != &v);
            }
        }
    }
    // declared flow-end rests (A13): ONLY resources with a §4 `(flow-end
    // rest)` output row return through a `rest_*` boundary exit — the
    // generator synthesizes none, so an undeclared rest keeps no exit and
    // surfaces at the model as a tripwire panic, the designed loud failure.
    let mut rest_budget: std::collections::BTreeMap<String, u64> = g
        .rest_exits
        .iter()
        .map(|r| (r.ident.clone(), r.count))
        .collect();
    let resting: Vec<(String, String)> = live
        .iter()
        .filter_map(|v| {
            let base = var_ty.get(v)?;
            let left = rest_budget.get_mut(base)?;
            if *left == 0 {
                return None; // more live than the §4 row declares: left loud
            }
            *left -= 1;
            Some((v.clone(), base.clone()))
        })
        .collect();
    for (v, base) in &resting {
        let _ = writeln!(s, "    rest_{}({v});", snake(base));
        live.retain(|x| x != v);
    }
    let final_budget = remaining;
    let _ = writeln!(
        s,
        "    let _person_back: Person<{}> = person;",
        lit(final_budget)
    );
    live.retain(|v| v != "person");
    let _ = writeln!(s, "    let _flow_end = ({}, history);", live.join(", "));
}

/// Emits the call and both-arm handling for an R17 fallible process in a
/// generated flow: the arm-specific products are routed to their §4 sinks
/// INSIDE the arm (so both arms account for everything), the conserved
/// participants continue past the match under fresh (never-shadowing) names.
fn emit_flow_fallible_call(
    g: &mut Generator,
    s: &mut String,
    plan: &ProcPlan,
    args: &str,
    live: &mut Vec<String>,
    var_ty: &mut std::collections::BTreeMap<String, String>,
    fresh: &dyn Fn(&[String], &str) -> String,
) {
    let (ok_name, fail_name) = bundle_names(plan);
    let ok_fields = bundle_fields(plan, true);
    let fail_fields = bundle_fields(plan, false);
    let common: Vec<(String, String)> = ok_fields
        .iter()
        .filter(|f| fail_fields.contains(f))
        .cloned()
        .collect();
    let only = |fields: &[(String, String)]| -> Vec<(String, String)> {
        fields.iter().filter(|f| !common.contains(f)).cloned().collect()
    };
    let ok_only = only(&ok_fields);
    let fail_only = only(&fail_fields);
    // each arm-only product's §4 sink (resolved up front: hole-reporting
    // below needs the generator mutably)
    let route = |g: &Generator, items: &[(String, String)]| -> Vec<(String, Option<String>)> {
        items
            .iter()
            .map(|(f, ty)| {
                let base = ty.split('<').next().unwrap_or(ty).trim().to_string();
                let sv = g
                    .sinks
                    .iter()
                    .find(|sk| sk.consumes.iter().any(|(i, _)| i == &base))
                    .map(|sk| snake(&sk.ident));
                (f.clone(), sv)
            })
            .collect()
    };
    let ok_route = route(g, &ok_only);
    let fail_route = route(g, &fail_only);
    let mut sinks_used: Vec<String> = Vec::new();
    for (_, sv) in ok_route.iter().chain(fail_route.iter()) {
        if let Some(sv) = sv {
            if !sinks_used.contains(sv) {
                sinks_used.push(sv.clone());
            }
        }
    }
    let attempt = fresh(live, "attempt");
    let _ = writeln!(s, "    let {attempt} = {}({args});", plan.fn_name);
    let _ = writeln!(
        s,
        "    // Both arms are handled (R17): each arm routes its own products to their"
    );
    let _ = writeln!(
        s,
        "    // §4 destinations; everything conserved in both arms continues past the match."
    );
    // fresh outer names for the conserved fields (never shadow a live value)
    let mut outer: Vec<String> = Vec::new();
    for (f, _) in &common {
        let mut pool = live.clone();
        pool.extend(outer.iter().cloned());
        outer.push(fresh(&pool, f));
    }
    let fmt_tuple = |v: &[String]| -> String {
        if v.len() == 1 {
            v[0].clone()
        } else {
            format!("({})", v.join(", "))
        }
    };
    let tuple_inner: Vec<String> = common
        .iter()
        .map(|(f, _)| f.clone())
        .chain(sinks_used.iter().cloned())
        .collect();
    let outer_all: Vec<String> = outer
        .iter()
        .cloned()
        .chain(sinks_used.iter().cloned())
        .collect();
    let _ = writeln!(s, "    let {} = match {attempt} {{", fmt_tuple(&outer_all));
    for (wrap, name, fields, route) in [
        ("Ok", &ok_name, &ok_fields, &ok_route),
        ("Err", &fail_name, &fail_fields, &fail_route),
    ] {
        let pat = fields
            .iter()
            .map(|(f, _)| f.clone())
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(s, "        {wrap}({name} {{ {pat} }}) => {{");
        for (f, sv) in route {
            match sv {
                Some(sv) => {
                    let _ = writeln!(s, "            let {sv} = send_to({sv}, {f});");
                }
                None => {
                    s.push_str(&hole_block(
                        g,
                        "            ",
                        &format!("no §4 destination resolved for the {wrap} arm's {f}"),
                        "This arm-only product matches no §4 sink by its canonical identifier, so\nthe scaffold cannot route it; it is bound loudly (the tripwire reports the\nleak at test time) until the implementer wires its destination.",
                    ));
                    let _ = writeln!(s, "            let unrouted_{f} = {f};");
                    let _ = writeln!(s, "            let _ = &unrouted_{f};");
                }
            }
        }
        let _ = writeln!(s, "            {}", fmt_tuple(&tuple_inner));
        let _ = writeln!(s, "        }}");
    }
    let _ = writeln!(s, "    }};");
    for (n, (_, ty)) in outer.iter().zip(common.iter()) {
        live.push(n.clone());
        let base = ty.split('<').next().unwrap_or(ty).trim().to_string();
        var_ty.insert(n.clone(), base);
    }
    // the stated waste leaves in both arms: route the conserved waste fields
    let rename: std::collections::BTreeMap<&String, &String> = common
        .iter()
        .map(|(f, _)| f)
        .zip(outer.iter())
        .collect();
    for w in &plan.waste {
        let Some(sink_var) = &w.sink_var else { continue };
        let Some(item) = &w.item else { continue };
        if w.duplicate_of_produce {
            continue;
        }
        let mut names: Vec<String> = Vec::new();
        if item.count > 1 {
            for i in 1..=item.count {
                names.push(format!("{}_{i}", snake(&item.ident)));
            }
        } else {
            names.push(snake(&item.ident));
        }
        for n in names {
            let Some(v) = rename.get(&n).map(|v| (*v).clone()) else {
                continue; // an arm-only waste was already routed in its arm
            };
            let _ = writeln!(s, "    let {sink_var} = send_to({sink_var}, {v});");
            live.retain(|x| x != &v);
        }
    }
}

// ── entry point ──────────────────────────────────────────────────────────

/// Emits the generated crate as (relative path, contents) pairs.
/// `model_core_path` is written verbatim into the generated Cargo.toml's
/// path dependency.
pub fn emit_crate(g: &mut Generator, pkg_name: &str, model_core_path: &str) -> Vec<(String, String)> {
    let crate_ident = pkg_name.replace('-', "_");
    let spec_name = g.spec.system_name.clone();
    let lib = emit_lib(g, &spec_name);
    let characteristics = emit_characteristics(g);
    let requirements = emit_requirements(g);
    // The `rest_*` exits are declared up front by the §4 `(flow-end rest)`
    // rows (A13, planned in `plan_boundary`), so flows and resources no
    // longer depend on emission order; the historical flows-first order is
    // kept for byte-stable output.
    let flows = emit_flows(g, &crate_ident);
    let resources = emit_resources(g);
    vec![
        ("Cargo.toml".into(), emit_cargo_toml(pkg_name, model_core_path)),
        ("src/lib.rs".into(), lib),
        ("src/characteristics.rs".into(), characteristics),
        ("src/requirements.rs".into(), requirements),
        ("src/resources.rs".into(), resources),
        ("tests/flows.rs".into(), flows),
    ]
}
