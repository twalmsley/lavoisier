# Model Specification — Fixture Beta (duplicate-REQ probe)

## 1. Purpose and scope

A minimal strict-clean specification; its REQ id deliberately collides with fixture beta's
(F-053: requirement ids are workspace-unique).

## 2. Requirements

**Ids:** REQ-101–REQ-101, allocated from the workspace sequence (F-053).

- **REQ-101:** The widget must be painted before it leaves.

## 3. Resources

| Resource | Kind | Characteristics (markers/parameters) | Quantity & unit | States |
|---|---|---|---|---|
| `Widget` | discrete | — | 1 | `raw` → `painted` |
| `Person` | reusable | time budget | 60_000 ms | budget draws down |

## 4. System boundary: suppliers, consumers, sinks

**Inputs (suppliers / sources):**

| What enters | Via | Capacity | Real or placeholder? |
|---|---|---|---|
| `Widget`, `Person` | setup at flow start | 1 each | placeholder |

**Outputs (consumers / sinks):**

| What leaves | Via | Capacity | Real or placeholder? |
|---|---|---|---|
| `painted` `Widget` | the customer | unbounded | placeholder |
| Expended time | History (R16) | unbounded | single History |

## 5. Processes

### P1. Paint the widget
- **Actor(s) and reusables:** person (draws 10_000 ms) — returned.
- **Consumes:** 1 `raw` `Widget`.
- **Produces:** 1 `painted` `Widget`.
- **Balances:** items 1 = 1 (structural); time 10_000 ms → History (structural).
- **Satisfies:** REQ-101.

## 6. Flows

- **Orders:** (a) P1; (b) P1.
- **Everything accounted:** the stamped widget at the customer; the person returned.

## 7. Assumptions and placeholders

- None.

## 8. Open questions for the author

- *(none)*
