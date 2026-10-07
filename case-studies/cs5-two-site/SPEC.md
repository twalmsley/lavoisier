# Model Specification — CS-5: Two-Site Fulfilment with Procurement

| | |
|---|---|
| Specification version | v0.1 |
| Date | 2026-10-07 |
| Author | Tony (drafted by Claude for review) |
| Status | draft |

## 1. Purpose and scope

A UK workshop fulfils a customer order for an instrument (a housing fitted with a precision
component) that must be **procured from an EU supplier and transported between sites**. This
is the boundary-refinement capstone: **two currencies with the bureau exchange** (R19's only
unexercised clause), **a placeholder refined into a real modelled supplier** (the R12
evolution path, finally walked), **transport between Locations** with one type per
location-state, **organisations as actors** (vendor, courier), and the model split across
**two team-shaped crates**.

**System boundary:** the works' world — its site, its account, its stock — plus the contracted
courier's loop to the EU vendor. The customer, the bureau and the vendor's own hinterland stay
at the boundary.

**Out of scope:** concurrency (CS-3), fallibility (CS-2 — transit never fails here), scale
(CS-4); customs and VAT; the courier's vehicle and fuel; lead times and scheduling (R9:
transit is a state change, not a duration); the vendor's sourcing.

## 2. Requirements

> Workspace ids: REQ-001..022 are taken; CS-5 starts at REQ-023.

- **REQ-023:** Foreign purchases must be paid in the supplier's currency (the vendor accepts
  euro cents only — a GBP payment must not compile).
- **REQ-024:** Currency may be exchanged only at the bureau, at its stated rate (R19:
  boundary-only, integer-exact).
- **REQ-025:** A component may be fitted only after goods-in inspection at the UK site.
- **REQ-026:** Delivery happens only against the customer's order, with payment taken on
  delivery.
- **REQ-027:** All inter-site transport must be by the contracted courier.

## 3. Resources

| Resource | Kind | Characteristics (markers/parameters) | Quantity & unit | States |
|---|---|---|---|---|
| Precision component | discrete w/ mass | 400 g; shipped boxed at 450 g | vendor stock: 3 (finite!) | at-vendor → in-transit (boxed) → at-UK (boxed) → inspected (400 g) |
| Packaging | waste | 50 g | per shipment | → works bin |
| Housing | discrete w/ mass | 600 g | works rack: 3 | — |
| Instrument | product | mass in type | 1000 g (600 + 400) | → customer |
| Pounds (GBP) | continuous (money) | pence | works account 5000 p; customer pays 9000 p | — |
| Euro cents (EUR) | continuous (money) | ec — **a separate dimension** | 2340 ec minted at the bureau | — |
| Exchange rate | bureau constant | 117 ec per 100 p (NUM/DEN) | fixed | — |
| Customer order | evidence token | sealed; REQ-026's key | 1 | placed → fulfilled |
| Works operator | reusable | time budget | 1 200 000 ms (20 min) | draws down |
| Courier | reusable **organisation** | `ContractedCourier` (REQ-027) | 1 | at-UK → outbound → at-vendor → inbound → at-UK |
| Vendor | boundary **organisation** | finite stock (the refined placeholder) | 1 | — |
| Works bin | contents-keeping consumer | decreasing space | capacity 5 | holds the packaging |

**Subsystem ownership (two team-shaped crates):**

| | `cs5-supply` (library) | `cs5-works` (depends on supply) |
|---|---|---|
| Owns | euro cents, the bureau + `exchange`, the vendor organisation and its 3-component stock, the courier and transit states, consign/carry/return transport processes, packaging | the GBP account, the housing rack, goods-in inspection, assembly, the customer + order token, delivery, the bin |
| REQs | REQ-023, REQ-024, REQ-027 | REQ-025, REQ-026 |

## 4. System boundary: suppliers, consumers, sinks

**Inputs:** the customer's order and 9000 p payment (boundary object); works stock (account
5000 p, 3 housings) and staff/courier at start; the bureau's 2340 ec minted against 2000 p
(R19 value-equivalence, boundary-only); the vendor's 3-component stock (its hinterland is the
boundary).

**Outputs:** the instrument + any change to the customer; 2340 ec to the vendor (exact
price); 50 g packaging to the works bin (emptied to disposal at flow end, F-039); expended
time to a single History; the works account ends at 12 000 p (5000 − 2000 + 9000).

**The refinement showcase (R12):** every earlier vendor was an unbounded `Next = Self`
placeholder. CS-5's vendor is the **refined** form: a modelled organisation whose stock is a
finite 3-deep supplier — a fourth purchase is a compile error, and exhaustion produces an
empty-stock state the vendor must account for. The bureau and customer deliberately **stay**
placeholders, so both shapes sit side by side; the vendor's rustdoc must show the
before/after contrast.

## 5. Processes

> Draws adjacent, recorded under process names; waste routing named; balances (assert)/(structural).

### P1. Exchange currency at the bureau — `cs5-supply` (REQ-024)
- Operator draws 120 000 ms. Account draw 2000 p (5000 → 3000, assert); bureau exchanges
  2000 p → 2340 ec (OUT × 100 = IN × 117, assert — exact by chosen amounts).
- **Satisfies:** REQ-024.

### P2. Consign the courier — `cs5-supply` (REQ-027)
- Operator draws 60 000 ms. The payment (2340 ec) and the purchase order are consigned to
  the contracted courier; courier state at-UK → outbound (structural).
- **Satisfies:** REQ-027.

### P3. Purchase at the vendor — `cs5-supply` (REQ-023, REQ-027)
- No operator time (the courier and vendor are the actors). The vendor consumes exactly
  2340 ec (REQ-023 structural: euro cents only, exact price) and supplies one boxed
  component (450 g) from its 3-stock; courier outbound → at-vendor → inbound with the box.
- **Balances:** money 2340 = 2340 (structural, exact-price impl); goods 1 = 1 (structural).

### P4. Receive at the works — `cs5-works`
- Operator draws 60 000 ms. Courier inbound → at-UK, surrendering the boxed component
  (450 g) at the works; courier returned for reuse.

### P5. Goods-in inspection — `cs5-works` (REQ-025)
- Operator draws 60 000 ms. Boxed component (450 g) → inspected component (400 g).
- **Waste routing:** packaging (50 g) fed to the works bin **inside the process**.
- **Balances:** mass 450 = 400 + 50 (assert).
- **Satisfies:** REQ-025 (produces its state).

### P6. Assemble the instrument — `cs5-works` (REQ-025)
- Operator draws 180 000 ms. Housing (600 g, from the rack) + inspected component (400 g) →
  instrument (1000 g).
- **Balances:** mass 600 + 400 = 1000 (assert).
- **Satisfies:** REQ-025 (accepts only the inspected state).

### P7. Deliver and take payment — `cs5-works` (REQ-026)
- Operator draws 120 000 ms. Consumes the instrument, the customer's **order token**
  (REQ-026's key) and the customer's 9000 p; instrument to the customer; 9000 p deposited
  (account 3000 → 12 000, assert).
- **Satisfies:** REQ-026.

### P8. Empty the bin — `cs5-works`
- Operator draws 30 000 ms. The bin's packaging (50 g) to disposal via the sealed F-039
  path; bin returned empty.
- **Balances:** mass 50 = 50 (assert via contents).

## 6. Flows

- P1 → P2 → P3 → P4 → P5 → P6 → P7 → P8, with the one genuine freedom: **the housing can be
  picked from the rack at any point before P6**, and P8 may run any time after P5 — at least
  two orderings compile. Total drawn: 630 000 ms (operator ends at 570 000 ms); History: 7
  attributed events (P3 has no draw).
- **Everything accounted:** instrument with the customer; account at 12 000 p; vendor paid
  (2340 ec, its dimension closed); vendor stock at 2 with its state accounted; courier home
  and reusable; packaging at disposal; bin empty; order token consumed; rack at 2; History
  with the caller.
- **The cross-currency books:** GBP and EUR never meet in one assert — each dimension
  balances separately, with the bureau's value-equivalence (R19) the only bridge, stated at
  its rate.

## 7. Assumptions and placeholders

- The bureau (fixed 117/100 rate) and the customer stay unbounded placeholders — the
  deliberate contrast to the refined vendor.
- Transit is a pure state change (R9); nothing is lost or delayed in carriage.
- The vendor keeps its revenue (boundary organisation; F-029 arithmetic-only accounting).
- Amounts are chosen so the exchange is remainder-free (2000 p × 117/100 = 2340 ec exactly);
  a non-exact amount would not compile (F-052) — stated as a feature.

## 8. Open questions for the author

1. **Two crates** (`cs5-supply` / `cs5-works`) rather than three (logistics separate) — the
   team-shaped split as proposed?
2. **Payment travels with the courier** (money consigned into transit, REQ-027) — the honest
   encoding of remote purchase. OK, or abstract the payment transfer?
3. **The refinement pair:** vendor refined to a finite 3-stock organisation; bureau and
   customer deliberately left as placeholders for contrast, with the before/after shown in
   the vendor's rustdoc. Right showcase?
4. **Exchange amounts chosen exact** (2000 p → 2340 ec at 117/100); the spec relies on
   F-052's "silent rounding is unrepresentable" rather than demonstrating a remainder split.
   OK, or add a deliberately non-exact probe as a compile_fail?
5. **Packaging + finite bin (5) + P8 disposal** — keeps CS-1's finite-sink shape in play. OK
   or drop for leanness?
6. **Single operator, single History; the courier and vendor act without time budgets**
   (organisations, not people). OK?
7. Quantities/prices sanity: §3's masses, 5000 p account, 9000 p sale, 2340 ec component,
   20-minute budget, 630 000 ms drawn. Happy?
