# Phase 3 — `orders` group

## Ops implemented (5)

| Method | Path | Rust fn |
|---|---|---|
| GET    | `/accounts/{accid}/orders`                       | `list_orders`    |
| POST   | `/accounts/{accid}/orders`                       | `place_order`    |
| PUT    | `/accounts/{accid}/orders/{order_id}`            | `modify_order`   |
| PUT    | `/accounts/{accid}/orders/{order_id}/activate`   | `activate_order` |
| DELETE | `/accounts/{accid}/orders/{order_id}`            | `cancel_order`   |

`GET /orders/{id}` is documented elsewhere but absent from the saved HTML
(per PROCESS.md Phase 1 finding) — only the 5 ops above are implemented.

## Doc inconsistencies / decisions

- **`FormData` vs JSON request body (`place_order`, `modify_order`) —
  RESOLVED.** The doc is unambiguous: every body parameter on these two
  endpoints is Swagger 2.0 `FormData`, which mandates
  `application/x-www-form-urlencoded`. The original wave-4 implementer
  picked JSON because the Phase 0 `Client` only exposed `post` / `put`;
  the reviewer flagged this and the user picked the correct fix:
  amend the foundation `Client` with `post_form` / `put_form` helpers.
  See `PROCESS.md` §"Locked decisions" item 9 for the amendment record
  (foundation tests `post_form_sends_application_x_www_form_urlencoded`
  and `put_form_sends_application_x_www_form_urlencoded` pin the wire
  format). The `orders` resource layer now uses `Client::post_form` for
  `place_order` and `Client::put_form` for `modify_order`. The wiremock
  tests assert both the `Content-Type: application/x-www-form-urlencoded`
  header and the exact urlencoded request body. Two side effects:
  - The request struct fields of type `Decimal`
    (`PlaceOrderRequest::price/target_value/trigger_value`,
    `ModifyOrderRequest::price`) intentionally omit the
    `rust_decimal::serde::arbitrary_precision_option` adapter — that
    adapter relies on a `serde_json`-private magic struct that
    `serde_urlencoded` rejects with `Error::EncodeForm("unsupported
    value")`. Default `Decimal` serde uses `Display`, which produces
    the decimal-string form (`101.5`) — correct for the wire.
  - As a result of the previous bullet, the request fixtures
    (`place_order.request.json`, `modify_order.request.json`) carry
    `Decimal` values as JSON strings (`"price":"101.5"`) rather than
    bare numbers. This keeps the canonical-roundtrip test green.
    Each fixture's `.meta.toml` records `wire_format =
    "application/x-www-form-urlencoded"` and explains the model-vs-wire
    relationship.
  Response `Decimal` fields (on `Order`, `ActivationCondition`, etc.)
  are unaffected — they continue to use `arbitrary_precision[_option]`
  because the response wire format is JSON.

- **Local `OrderAmount` instead of shared `Amount` / `Money`.**
  `_definitions/Amount.md` describes the wire shape as
  `{currency, value}` (note: `value`, not `amount`). The shared
  [`crate::models::shared::Amount`] is a transparent newtype over
  `Decimal` (no nested object). The shared
  [`crate::models::shared::Money`] uses `{currency, amount}` (different
  field name). Neither matches the documented `Amount` schema verbatim,
  so a local [`OrderAmount`] type is declared. Flagged for Phase 3X
  reconciliation — likely the right fix is to rename or extend the
  shared `Amount`.

- **`Order.order_type` typed as `String`.** `_definitions/Order.md`
  documents `order_type` only as `string` — no enumerated value set on
  the response side. The place-order request enum
  [`OrderType`] has a documented value set
  (`FAK, FOK, NORMAL, LIMIT, STOP_LIMIT, STOP_TRAILING, OCO`). We keep
  the response-side value as `String` to admit any value the server
  might report — defensive against drift between the request enum and
  response value.

- **Two distinct `ActivationCondition` shapes.** The request side
  ([`OrderActivationCondition`]) is an enum with documented values
  `STOP_ACTPRICE_PERC, STOP_ACTPRICE, MANUAL, OCO_STOP_ACTPRICE`. The
  response side ([`ActivationCondition`]) is a struct with `type`
  (enum) plus `trailing_value`, `trigger_value`, `trigger_condition`.
  The response `type` enum ([`ActivationConditionType`]) additionally
  documents a `NONE` variant. Both kept distinct.

- **Local `OrderType` enum vs `tradables::OrderType` struct.** Two
  unrelated concepts that share a name. Each lives in its own module so
  there is no symbol clash. Phase 3X reconciliation candidate.

- **Multi-id paths (`accid`, `order_id` for `activate_order`).** The
  Nordnet path slots accept comma-separated lists. The typed surface
  here stays single-id by default. `activate_order` already returns a
  `Vec<OrderReply>` per the docs (a single call still receives a
  one-element array).

- **`Order.modified` and `Validity.valid_until`.** Documented as
  `integer(int64)` UNIX millisecond epochs. Kept as plain `i64` —
  consistent with `tradables::PublicTrade.tick_timestamp` precedent.
  No `EpochMillis` newtype in `crate::models::shared`.

- **404 on `modify_order`.** Doc lists 404 as a possible status; the
  foundation [`Error`] enum has no dedicated `NotFound` variant, so it
  maps to [`Error::UnexpectedStatus`]. Not modified — the foundation
  layer is locked.

## Files written

- `crates/nordnet-api/src/models/orders.rs`
- `crates/nordnet-api/src/resources/orders.rs`
- `crates/nordnet-api/tests/orders_test.rs`
- `crates/nordnet-api/fixtures/orders/list_orders.{response.json,meta.toml}`
- `crates/nordnet-api/fixtures/orders/place_order.{request.json,response.json,meta.toml}`
- `crates/nordnet-api/fixtures/orders/modify_order.{request.json,response.json,meta.toml}`
- `crates/nordnet-api/fixtures/orders/activate_order.{response.json,meta.toml}`
- `crates/nordnet-api/fixtures/orders/cancel_order.{response.json,meta.toml}`

## Test coverage

- 7 fixture roundtrip tests (one per fixture, including request
  bodies).
- 2 `deny_unknown_fields` rejection tests.
- Wiremock success + 401/403 / 400 error-mapping tests for every op
  (14 wiremock tests). For the FormData write ops (`place_order`,
  `modify_order`) the success test asserts both
  `Content-Type: application/x-www-form-urlencoded` and the exact
  urlencoded request body via `body_string` (constants
  `PLACE_ORDER_EXPECTED_FORM_BODY` / `MODIFY_ORDER_EXPECTED_FORM_BODY`
  near the top of `tests/orders_test.rs`).
- Body-less PUT verified for `activate_order` via `body_bytes(b"")`.
- `list_orders` 204 mapped to empty `Vec`.

23 tests total, all green.

## Open questions

- ~~The `FormData` vs JSON question above.~~ Resolved — see the
  decision above. Foundation amended with `post_form` / `put_form`
  (PROCESS.md §"Locked decisions" item 9); orders uses them.
- Long-term reconciliation of the local `OrderAmount` against the
  shared `Amount` / `Money` types is Phase 3X work.
