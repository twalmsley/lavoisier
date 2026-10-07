# Model Specification — CS-2: Bicycle Puncture Repair

| | |
|---|---|
| Specification version | v0.3 (implemented) |
| Date | 2026-10-07 |
| Author | Tony (drafted by Claude; reviewed and agreed 2026-10-07) |
| Status | agreed |

## 1. Purpose and scope

One person repairs a punctured rear wheel at a community bike workshop. Chosen as the second
case study because it exercises, in a household-recognisable domain, the three features CS-1
deliberately avoided: **fallible processes with provisioned rework** (a patch can fail — R17),
**qualification** (workshop tools may only be used by inducted members — R18), and **money**
(a spare tube is purchased at the parts counter — R19). The repair kit is a nested container
(discrete patches plus a continuous cement tube inside one kit).

**System boundary:** the workshop bay. A punctured wheel, the member with their kit and cash,
and the workshop's tools enter it; a serviceable wheel, waste, and (on one path) a dead tube
leave it.

**Out of scope:** removing the wheel from the bicycle (the wheel arrives off the bike);
diagnosing *why* it punctured; tyre wear; the workshop's business (the counter is a boundary
vendor); scheduling; temperature/cure time (R9 — states stand in for "cured").

## 2. Requirements

> Workspace ids: pilot owns REQ-001..005, CS-1 owns REQ-006..009; CS-2 starts at REQ-010.

- **REQ-010:** Workshop tools (the workstand) may be used only by an inducted member.
- **REQ-011:** Only a located puncture may be patched (no blind patching).
- **REQ-012:** A wheel may be refitted only with an airtight tube — one that is *patched and
  checked*, or *new*. (Deliberately the first requirement satisfied by **two** types.)
- **REQ-013:** All failed patches must reach the workshop waste stream.
- **REQ-014:** Spare tubes are sold only at the counter's listed price.

## 3. Resources

| Resource | Kind | Characteristics (markers/parameters) | Quantity & unit | States |
|---|---|---|---|---|
| Wheel | product | mass carried in the type | 1900 g (rim + tyre) + tube | punctured → open (tube out) → serviceable |
| Inner tube | discrete w/ mass | mass; `Airtight` marker on ready states | 180 g | punctured → located-puncture → patched(183 g) → checked/ready; or dead (→ recycling) |
| Patch | discrete | 2 g each | kit holds 2 | dry → spent(3 g, incl. cement) |
| Cement | continuous | grams | 30 g tube, 1 g per application | — |
| Patch kit | nested container | holds the 2 patches **and** the cement tube | 1 | draws down; remainder stays with the member |
| Spare tube | discrete w/ mass | `Airtight` (new) | 180 g, price 650 p | — |
| Cash | continuous (money) | pence | 1000 p in the wallet | split 650 + 350 on purchase |
| Member (person) | reusable | time budget; `Inducted` qualification (R18 wrapper) | 1 800 000 ms (30 min) | budget draws down |
| Workstand, pump | reusable | workstand gated by REQ-010 | 1 each | — |
| Patch outcome | outcome token (R17) | sealed, boundary-injected | 2 provisioned | — |

## 4. System boundary: suppliers, consumers, sinks

**Inputs (suppliers / sources):**

| What enters | Via | Capacity | Real or placeholder? |
|---|---|---|---|
| Punctured wheel | service intake | 1 | placeholder: the member's bike |
| Member, kit, cash, tools | bay setup at flow start | 1 each | placeholder |
| Induction | `qualify` boundary process (R18) | — | placeholder: induction body |
| Spare tube | parts counter (vendor: consumes exact price, supplies tube, `Next = Self`) | unbounded | placeholder: workshop stores |
| Patch outcomes | boundary constructors / test fixtures | 2 | placeholder until calibrated |

**Outputs (consumers / sinks):**

| What leaves | Via | Capacity | Real or placeholder? |
|---|---|---|---|
| Serviceable wheel | the owner | unbounded | placeholder |
| Failed (spent) patches | workshop waste stream | unbounded (`Next = Self`) | placeholder — deliberately the *other* sink shape from CS-1's finite bin |
| Dead tube (path 3 only) | rubber recycling | unbounded | placeholder |
| Payment (path 3 only) | parts counter | — | the vendor consumer above |
| Expended time | History (R16) | unbounded | single History (one actor); the per-branch merge demo remains deferred to CS-3 |
| Untried outcome tokens | boundary exit | — | returned to the environment (R17) |

## 5. Processes

> Per the template conventions: actor draws are adjacent `draw_time` steps recorded under the
> process name; waste routing is named per output; balances are marked (assert)/(structural).

### P1. Open the wheel (remove the tube)
- **Actor(s) and reusables:** member (draws 240 000 ms), workstand — returned. **REQ-010
  bounds the member.**
- **Consumes:** punctured wheel (2080 g).
- **Produces:** open wheel (rim + tyre, 1900 g) + punctured tube (180 g).
- **Balances:** mass 2080 = 1900 + 180 (assert); time → History (structural).
- **Satisfies:** REQ-010.

### P2. Find the hole
- **Actor(s) and reusables:** member (draws 180 000 ms), pump (inflate to listen/feel) — returned.
- **Consumes:** punctured tube (180 g).
- **Produces:** located-puncture tube (180 g — a state change, no mass change).
- **Balances:** mass 180 = 180 (structural); time → History (structural).
- **Satisfies:** — (enables REQ-011).

### P3. Patch the tube — **fallible (R17)**
- **Actor(s) and reusables:** member (draws 120 000 ms per attempt) — returned in both arms.
- **Consumes:** located-puncture tube (180 g), 1 patch (2 g) from the kit, 1 g cement drawn
  from the kit's cement tube, 1 outcome token.
- **Produces (Ok):** patched tube (183 g).
- **Produces (Fail):** the tube, still with its located puncture (180 g) + 1 spent patch (3 g).
- **Waste routing:** the spent patch is fed to the workshop waste stream **inside the
  process** (requirement-bounded consumer parameter), so failed patches never exist loose —
  REQ-013 structural.
- **Balances (both arms):** mass 180 + 2 + 1 = 183 (assert, Ok) and 180 + 2 + 1 = 180 + 3
  (assert, Fail); cement 1 g per attempt (assert, kit draw); time → History (structural).
- **Satisfies:** REQ-011 (accepts only the located-puncture state), REQ-013.
- **Failure modes:** patch fails to seal. Provisioned rework: the kit holds 2 patches and 2
  outcome tokens are provisioned, so **at most two attempts** are expressible (R17/F-050).

### P4. Check the patch
- **Actor(s) and reusables:** member (draws 60 000 ms), pump — returned.
- **Consumes:** patched tube (183 g).
- **Produces:** checked, ready tube (183 g) — the `Airtight` patched state (REQ-012).
- **Balances:** mass 183 = 183 (structural); time → History (structural).
- **Satisfies:** — (enables REQ-012).

### P5. Buy a spare tube — path 3 only (R19)
- **Actor(s) and reusables:** member (draws 120 000 ms) — returned.
- **Consumes:** 1000 p cash, split 650 + 350 (assert); 650 p to the counter.
- **Produces:** spare tube (180 g, `Airtight` new), 350 p change (stays in the wallet).
- **Waste routing:** the dead punctured tube goes to rubber recycling (flow-routed).
- **Balances:** money 1000 = 650 + 350 (assert); goods 1 = 1 (structural); time → History (structural).
- **Satisfies:** REQ-014 (the counter consumes only `Money<650>` — structural, exact-price impl).

### P6. Refit and inflate
- **Actor(s) and reusables:** member (draws 300 000 ms), workstand, pump — returned. **REQ-010
  bounds the member; REQ-012 bounds the tube.**
- **Consumes:** open wheel (1900 g) + an `Airtight` tube (183 g patched / 180 g spare).
- **Produces:** serviceable wheel (2083 g on the patched paths; 2080 g with the spare).
- **Balances:** mass 1900 + tube = wheel (assert, per instantiation); time → History (structural).
- **Satisfies:** REQ-010, REQ-012.

## 6. Flows

- Dependencies: P1 → P2 → P3; P3-Ok → P4 → P6; P3-Fail → (second P3) or (P5 → P6). P6 ends
  every path.
- **Three paths** (the flow returns a `#[must_use]` outcome grouping, one variant per path,
  per R17/F-050 — the paths end with *different* wheel masses and wallet states, so the
  variants carry different types):
  1. **Patched first try:** P1 P2 P3(Ok) P4 P6 — 900 000 ms drawn (5 attributed events);
     wallet 1000 p; 1 spare patch and 1 untried token returned to kit/boundary; 29 g cement
     left.
  2. **Patched on retry:** P1 P2 P3(Fail) P3(Ok) P4 P6 — 1 020 000 ms (6 events); 1 spent
     patch (3 g) in the waste stream; kit empty of patches; 28 g cement left.
  3. **Spare fitted:** P1 P2 P3(Fail) P3(Fail) P5 P6 — 1 080 000 ms (6 events); 6 g waste;
     dead tube (180 g) to recycling; wallet 350 p; wheel 2080 g.
- **Ordering freedom is deliberately minimal here** (one actor, a linear repair): the only
  freedoms are trivia such as when the wallet is readied. That is expected — CS-2's stress is
  fallibility, not concurrency, which remains CS-3's job. Both statically expressible
  orderings of P5 relative to the second P3-Fail on path 3 must still compile.
- **Everything accounted at every path's end:** wheel at the owner; member back with their
  remaining budget; workstand and pump back; kit back with its remaining patches and cement;
  unspent cash in the wallet; spent patches in the waste stream; untried tokens returned via
  the boundary exit; dead tube (path 3) at recycling; the single History holding one
  attributed event per draw.

## 7. Assumptions and placeholders

- The workshop waste stream and rubber recycling are unbounded placeholder sinks.
- The parts counter never runs out of spare tubes (`Next = Self`) and is a placeholder.
- Induction is a placeholder boundary process (no training modelled); the member arrives
  with cash already in hand.
- Patch/cement/time figures are round-number stand-ins, not calibrated measurements.
- Two patches and two outcome tokens bound the rework (R17): a third attempt is deliberately
  inexpressible.

## 8. Open questions for the author

Review decisions (2026-10-07): questions 1–4, 6 and 7 — **agreed as proposed** (induction
framing; unbounded waste stream with no disposal step; single History; per-path end states as
outcome-grouping variants; the round quantities; REQ-012 stays multi-type). Question 5 —
**changed:** P4 gets its own 60 000 ms draw (path totals updated above to 900 000 /
1 020 000 / 1 080 000 ms).

Implementation round-trip feedback (2026-10-07):
1. **A state change must have an owning process:** §5 routed the dead tube to recycling but no
   process owned located→dead; the model needed an explicit `retire_tube` (R9). Folded into
   the flows; future specs should name every conversion.
2. **Nested-kit encoding:** the cement tube is the kit type's own continuous magnitude
   (`PatchKit<Patches, CEMENT_G>`), drawn through the kit's boundary — not a held container
   object (F-040 forbids tripwired held contents). Both kinds of contents remain facts of one
   sealed kit type.
3. **Unbounded sinks discard (F-029):** the "3 g / 6 g in the waste stream" end states are
   assertable only arithmetically from the constants, not by inspecting the sink.
4. **Ordering precision:** the expressible P5 orderings are around wallet-readying and
   dead-tube retirement; buying the spare *before* the second failure is deliberately
   inexpressible (F-050 extension) — a feature, now stated as one.
5. **REQ-012's multi-type satisfaction** exposed the F-055 point-3 diagram ambiguity as a
   real silent error; the generator now draws one edge per satisfying type (fixed same day).

Original questions, for the record:

1. **Qualification framing:** R18 is exercised as *workshop induction* gating the workstand
   (REQ-010) — honest for a community workshop, and avoids pretending home repairs need
   certificates. OK?
2. **The waste stream is unbounded** (the opposite sink shape from CS-1's finite bin, so both
   shapes are now demonstrated) and there is **no disposal step** this time. OK?
3. **Single History again** (one actor; merge demo stays with CS-3). OK?
4. **Per-path end states:** the three paths finish with different wheel masses (2083/2083/2080)
   and wallet states, carried as different variants of the flow's outcome grouping (F-050).
   OK, or would you rather the spec force a uniform end state (e.g. model the mass difference
   away)?
5. **P4's time:** the check is folded into P3's 120 000 ms per attempt rather than drawing
   separately — keeps the arithmetic simple. OK, or give P4 its own draw?
6. Quantities sanity check: 1900 g rim+tyre, 180 g tubes, 2 g patches + 1 g cement, 30 g
   cement tube, 650 p spare from a 1000 p wallet, 30-minute budget. Happy?
7. **REQ-012 is deliberately multi-type** (patched-and-checked *or* new spare satisfy it) —
   the first real test of the F-055 note that `satisfies!` resolution becomes many-to-many.
   Keep it so?
