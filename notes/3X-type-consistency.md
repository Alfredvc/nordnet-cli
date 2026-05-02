# Phase 3X — Cross-endpoint type consistency

Reviewer: opus, single-pass after Phase 3 wave 4 landed.
Test/lint state at exit: `cargo test --workspace -j 2` → 264 passed, 0 failed
(258 pre-3X plus 6 new tests in `models/shared.rs`); `cargo clippy
--workspace --all-targets -- -D warnings` clean; `cargo fmt --all --check`
clean.

## Method

Built a name-keyed map of every public `struct`/`enum`/`type` declared
under `crates/nordnet-api/src/models/*.rs` (110 declarations in 13 group
files). For each name with >1 definition, diffed the declarations and
applied the Phase 3X rule from `PROCESS.md`:

- **Identical, ≥3-group dup → promote** to `models/shared.rs` (or
  `crate::ids` for newtypes).
- **Identical, 2-group dup → leave** in place (avoid `shared.rs` churn).
- **Different, ≥2-group → reconcile** under conservative pick (union of
  fields, `Option<T>` for fields not present in all definitions; document
  in `Order.md`-style notes).

Also acted on the explicit known-issues list from the Phase 3X prompt:
`IssuerId` promotion, `PublicTrade` dup, `Tradable.identifier`,
`expiration_date`, `OrderAmount` vs shared `Money`/`Amount`, the four
`opt_arb_prec` adapter copies.

## Type-name → action map

| Name | Where defined (before 3X) | Diff | Action |
|---|---|---|---|
| `opt_arb_prec` (mod) | 4 groups: accounts, instruments, instrument_search, main_search | Byte-identical | **Promoted** to `crate::models::shared::opt_arb_prec`. All 4 group files now `use crate::models::shared::opt_arb_prec;` and reference it via `#[serde(with = "opt_arb_prec")]`. |
| `IssuerId` | Local newtype in `models::instruments` (used in `instruments.rs`); `instrument_search::InstrumentInfo.issuer_id` carried plain `i64` for lack of the newtype. | Single canonical newtype shape (`pub i64`); the `i64` in `instrument_search` was a pure type-correctness gap. | **Promoted** to `crate::ids::IssuerId`. `instrument_search::InstrumentInfo.issuer_id` now `Option<IssuerId>`. Local newtype removed from `models::instruments`. |
| `AmountWithCurrency` (was `accounts::Amount` + `orders::OrderAmount`) | `accounts::Amount { currency: String, value: Decimal }`; `orders::OrderAmount { currency: Currency, value: Decimal }` | Same wire shape (`{currency, value}` per `_definitions/Amount.md`); `currency` field type differed (`String` vs `Currency` newtype). | **Reconciled + promoted** under the "field-shape divergence" carve-out. New `crate::models::shared::AmountWithCurrency { currency: Currency, value: Decimal }`. `accounts::Amount` and `orders::OrderAmount` are now `pub use` aliases of the shared type, normalizing on the `Currency` newtype. Wire-compatible (`Currency` is `serde(transparent)` over `String`). Test sites in `accounts_test.rs` updated to compare against `Currency::from("...")`. |
| `Tradable.identifier` | `instruments::Tradable { identifier: String, ... }` | Single-site, but the docs-extract `_definitions/Tradable.md` schema expresses the exact `identifier` concept that `crate::ids::TradableId` newtype models. | **Type-swap.** `String` → `crate::ids::TradableId`. Wire-identical (`TradableId` is `serde(transparent)`). |
| `expiration_date: Option<String>` | `instruments::Instrument`, `accounts::PositionInstrument` | Both carry doc-marked `string(date)` (YYYY-MM-DD); previously left as `String` with TODO notes pointing at Phase 3X. | **Type-swap.** Both → `Option<time::Date>` via the new `crate::models::shared::date_iso8601::option` adapter. |
| `registration_date: Option<String>` | `accounts::AccountInfo` | Same `string(date)` pattern as `expiration_date`. | **Type-swap** to `Option<time::Date>` via `date_iso8601::option`. |
| `LeverageFilter.expiration_dates: Vec<String>` | `instruments::LeverageFilter` | Vec of `string(date)`. | **Type-swap** to `Vec<time::Date>` via the new `date_iso8601::vec` adapter. |
| `date_iso8601` (mod) | New | n/a | **Added** to `models/shared.rs` (used by 3 groups: accounts, instruments — `tradables::CalendarDay.date` left as `String`; see "Flagged but not fixed"). |
| `PublicTrade` | instruments + tradables | Byte-identical (same fields, same types, same attrs, same `cannot derive Eq` reason). | **Left duplicated.** 2-group dup, no field-shape divergence — the rule says leave it. Documented in both files' module headers. |
| `OrderType` | orders + tradables | Fundamentally different: `orders::OrderType` is an `enum` (place_order request enum); `tradables::OrderType` is a `struct { name, type }` (allowed-order-type entry on `TradableInfo`). Same name by accident; live in distinct modules so no symbol clash. | **Left as-is.** Documented in `orders.rs` module header. Renaming either would either break the request enum's natural English name (`OrderType` is exactly what the Nordnet docs call it on `place_order`) or invent a new name for `tradables`. No reconciliation possible. |
| `ActivationCondition` | orders only — but in two shapes: `struct ActivationCondition` (response, nested in `Order`) and `enum OrderActivationCondition` (request, sent as form field). | Inherently asymmetric per docs — the response shape carries `trailing_value`, `trigger_value`, `trigger_condition`, plus `type`; the request shape is just the enum value. The docs document them as different things under the same conceptual umbrella. | **Both kept as documented.** Already named distinctly (`ActivationCondition` vs `OrderActivationCondition`) and the response struct's `type` field uses the response-only enum `ActivationConditionType` (which has the additional `NONE` variant the request enum lacks). No further action. |
| `Currency` | shared only | n/a | No change. Two groups (`accounts::AccountInfo.account_currency`, `accounts::Ledger.currency`) deliberately use bare `String` because the docs document those fields as bare `string` rather than as a separate currency object — left as `String` per the conservative-pick rule. |
| `EtpInfo`, `KoInfo`, `MarketInfo`, `PriceKoInfo`, `PriceWithDecimals` | instrument_search + main_search | Byte-identical (all 5). | **Left duplicated.** 2-group dup, no field-shape divergence. Documented in both module headers. |
| `Money` | shared only | Unused (only mentioned in doc-comments). | **Left as-is.** Removing it would touch the foundation lock surface for no functional gain; kept and documented as "currently unused". |
| `Amount` (shared transparent newtype `Decimal`) | shared only | Unused. | **Left as-is.** Distinct shape from `AmountWithCurrency` (no `currency` field); kept for the case where a bare-numeric "money is decimal" wire field shows up. |
| `PositionInstrument` (`accounts`) vs `Instrument` (`instruments`) | 2 groups | Different field sets — `Position.instrument` lacks `tradables`, `underlyings`, `key_information_documents`, `mifid2_category`, `sfdr_article`, `total_fee`, `prospectus_url`, `brochure_url`'s subset of meta. The schemas are documented separately in the docs-extract definitions. | **Left as-is.** Reconciling under union-with-`Option<T>` would force the consolidated type to drop `deny_unknown_fields` evidence on response payloads of either group, undoing the strict-type contract. Documented in `accounts.rs` module header. |
| `TradableRef` (`accounts`) vs `OrderTradable` (`orders`) | 2 groups | Same wire shape `{identifier, market_id}`; different names. | **Left duplicated.** 2-group dup, no field-shape divergence. Names match the doc text in their respective groups (`TradableId` schema object in `accounts/Trade.md`; nested `tradable` object in `orders/Order.md`). Documented in both files. |
| `NewsId`, `NewsSourceId` | news only | Single-group local newtypes. | **Left as-is.** Single use site; no promotion needed. |
| `TradableKey` (`tradables`) | tradables only | Composite key for path-slot construction (`market_id:identifier`); not a wire shape. | **Left as-is.** Single-site helper. |

## Files touched

Code:
- `crates/nordnet-api/src/models/shared.rs` — added `AmountWithCurrency`, `opt_arb_prec`, `date_iso8601` (with `option` and `vec` submodules), and 7 new unit tests.
- `crates/nordnet-api/src/ids.rs` — added `IssuerId` newtype.
- `crates/nordnet-api/src/models/accounts.rs` — `pub use` of shared `AmountWithCurrency` as `Amount`; `registration_date` and `PositionInstrument.expiration_date` switched to `Option<time::Date>`; removed local `opt_arb_prec`; updated module doc notes.
- `crates/nordnet-api/src/models/instruments.rs` — removed local `IssuerId` and `opt_arb_prec`; `Tradable.identifier` → `TradableId`; `Instrument.expiration_date` → `Option<time::Date>`; `LeverageFilter.expiration_dates` → `Vec<time::Date>`; updated imports + module doc notes.
- `crates/nordnet-api/src/models/instrument_search.rs` — removed local `opt_arb_prec`; `InstrumentInfo.issuer_id` → `Option<IssuerId>`; updated imports + module doc notes.
- `crates/nordnet-api/src/models/main_search.rs` — removed local `opt_arb_prec`; updated imports + module doc notes.
- `crates/nordnet-api/src/models/orders.rs` — removed local `OrderAmount` struct; replaced with `pub use crate::models::shared::AmountWithCurrency as OrderAmount`; updated module doc notes.
- `crates/nordnet-api/src/resources/instruments.rs` — `IssuerId` import moved from `models::instruments` to `crate::ids`.

Tests:
- `crates/nordnet-api/tests/instruments_test.rs` — `IssuerId` import from `crate::ids`; `expiration_date` assertion uses `time::macros::date!`; `Tradable` construction wraps `identifier` in `TradableId`.
- `crates/nordnet-api/tests/instrument_search_test.rs` — `IssuerId` import from `crate::ids`; `issuer_id` assertion wraps in `IssuerId`.
- `crates/nordnet-api/tests/accounts_test.rs` — added `Currency` import and `time::macros::date!`; `Amount.currency` comparisons updated to `Currency::from("...")`; `registration_date` assertion uses `date!`; one `currency: "SEK".to_owned()` literal switched to `.into()` (since `Amount.currency` is now `Currency`, not `String`).

Docs:
- `notes/3X-type-consistency.md` — this file (new).
- `PROCESS.md` — added §"Locked decisions" item 11 documenting the `shared.rs` extension and `crate::ids::IssuerId` promotion.

## Flagged but not fixed (intentionally)

1. **`tradables::CalendarDay.date: String`** — also a `string(date)` schema field. Not switched to `time::Date` because `CalendarDay` carries `Eq` derive (the rest of `tradables` types do). `time::Date` derives `Eq` so the switch would compile, but the change is not in the explicit Phase 3X known-issues list; leaving it minimizes churn in a green group. Recommend a Phase 4 follow-up if a CLI consumer needs to compare dates structurally.
2. **`PositionInstrument` ↔ `instruments::Instrument`** — overlapping but different field sets. Consolidation would either weaken `deny_unknown_fields` (drop fields), or surface speculative `Option<T>` fields in groups that have no schema evidence for them. Documented as a known divergence in `accounts.rs` module header. No immediate action.
3. **`shared::Money` and `shared::Amount`** — both unused in current code. Per the foundation-lock rule (Phase 0 owns `shared.rs`'s original surface), I did not delete them. They remain in `shared.rs` with comments documenting them as "currently unused, kept against future doc revisions".
4. **`AccountInfo.account_currency: String`, `Ledger.currency: String`** — left as `String` per docs (bare `string` schema, not the `Currency` shape used in `Amount`). Phase 3X reviewer should reconfirm if a future doc revision aligns these.
5. **`NewsId`, `NewsSourceId`** — single-group local newtypes. Not promoted because each is used in exactly one group. (`models::main_search::MainSearchResponseRow.external_news_id` is documented as `integer(int64)` and was the only candidate for sharing, but `news::NewsId` is private to the news module; promoting would require deeper reasoning about ID semantics across two groups for one field. Documented as a future-cleanup item.)
6. **`Order.modified` etc. — UNIX-millis epoch fields.** No `EpochMillis` newtype exists in `crate::models::shared`. Several groups carry these as plain `i64`. Promoting to a typed adapter is out of scope for Phase 3X (no doc demand); flagged as a Phase 5 / post-release follow-up.

## New `shared.rs` surface (post-3X)

```text
pub struct ErrorResponse           // unchanged from Phase 0
pub struct Currency                // unchanged from Phase 0
pub struct Money                   // unchanged from Phase 0 (unused)
pub struct Amount                  // unchanged from Phase 0 (unused; transparent Decimal)
pub struct AmountWithCurrency      // ADDED by 3X (used by accounts + orders)
pub type   Timestamp               // unchanged from Phase 0
pub mod    opt_arb_prec            // ADDED by 3X (used by 4 groups)
pub mod    date_iso8601            // ADDED by 3X (used by 2 groups: accounts, instruments)
   ↳ mod option                    // Option<Date> flavor
   ↳ mod vec                       // Vec<Date> flavor
```

## New `ids.rs` surface (post-3X)

```text
pub struct AccountId(pub i64);     // Phase 0
pub struct OrderId(pub i64);       // Phase 0
pub struct InstrumentId(pub i64);  // Phase 0
pub struct MarketId(pub i64);      // Phase 0
pub struct TickSizeId(pub i64);    // Phase 0
pub struct TradableId(pub String); // Phase 0
pub struct IssuerId(pub i64);      // ADDED by 3X
```
