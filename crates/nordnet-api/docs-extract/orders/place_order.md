# place_order — Enter order

## Endpoint

`POST /api/2/accounts/{accid}/orders`

## Description

Enters a new order for the tradable identified by the given market ID + tradable ID.

## Parameters

| Type | Name | Description | Schema | Default |
|------|------|-------------|--------|----------|
| Header | Accept-Language optional | Language preferred in the response. Overrides the session language. Note that nb and nn are equivalent to no. | enum (da, de, en, fi, nb, nn, no, sv) |  |
| Header | Authorization optional | Basic authorization header, that is the value must be Basic <credentials> where <credentials> is the base64 encoded string <session_id>:<session_id>. | string |  |
| Path | accid required | The account identifier for the account. Some systems can or must use the account number as account identifier. | integer(int64) |  |
| FormData | activation_condition optional | Used for stop-loss orders. STOP_ACTPRICE_PERC - Trailing stop-loss. The order is activated when the price changes by the given percentage. The fields target_value, trigger_value and trigger_condition are required and the price field should be omitted. STOP_ACTPRICE - The order is activated when the market price of the instrument reaches a trigger price. The fields trigger_value, trigger_condition and price are required. MANUAL - The order is inactive in the Nordnet system until it is manually activated by the customer. OCO_STOP_ACTPRICE - One cancels other orders. Our implementation is one normal order and one stop-loss order. If the real order is executed the stop-loss is cancelled. If the stop-loss triggers the normal order is canceled. This combination is always displayed as one order. | string |  |
| FormData | currency optional | The currency that the instrument is traded in. | string |  |
| FormData | extended_hours optional | If true, order is applicable for US pre-market trading. Defaults to false. | boolean |  |
| FormData | identifier optional | Nordnet tradable identifier. | string |  |
| FormData | market_id required | Nordnet market identifier. | integer(int64) |  |
| FormData | open_volume optional | The visible part of an iceberg order. If this field is omitted the whole volume of the order is visible on the market. This field is only allowed if the order type is LIMIT or NORMAL. | integer(int64) |  |
| FormData | order_type optional | The order type. Defaults to NORMAL. When using NORMAL the system guesses the order type based on used parameters. For better parameter validation and to ensure that the order type is the desired one the client should not use NORMAL, but rather the intended order type instead. NORMAL will be deprecated in future versions. | enum (FAK, FOK, NORMAL, LIMIT, STOP_LIMIT, STOP_TRAILING, OCO) | "NORMAL" |
| FormData | price optional | The price limit of the order. | number(double) |  |
| FormData | reference optional | Free text reference for the order. Intended for the customer. | string |  |
| FormData | side required | BUY or SELL. | enum (BUY, SELL) |  |
| FormData | target_value optional | Only used when type is STOP_ACTPRICE_PERC or OCO_STOP_ACTPRICE. This is the price on the market. If type is STOP_ACTPRICE_PERC the value is given in percentage points. The price will be trailing_value + (target_value percentage of trailing_value). If type is OCO_STOP_ACTPRICE the price is a fixed price. | number(double) |  |
| FormData | trigger_condition optional | The comparison that should be used on trigger_value. Valid values are <= (less than or equal to) or >= (greater than or equal to). | string |  |
| FormData | trigger_value optional | If type is STOP_ACTPRICE_PERC the value is given in percentage points. Minimum value is 1 for STOP_ACTPRICE_PERC. If type is STOP_ACTPRICE the value is a fixed price. | number(double) |  |
| FormData | valid_until optional | Date formatted as YYYY-MM-DD. If this field is left out the order is a day order - that is, the same behavior as if valid_until is set to today. Smart-orders can only be day orders. | string(date) |  |
| FormData | volume required | The volume of the order. | integer(int64) |  |

## Request Body Schema

_(form data parameters)_

- **activation_condition optional** (string) — Used for stop-loss orders. STOP_ACTPRICE_PERC - Trailing stop-loss. The order is activated when the price changes by the given percentage. The fields target_value, trigger_value and trigger_condition are required and the price field should be omitted. STOP_ACTPRICE - The order is activated when the market price of the instrument reaches a trigger price. The fields trigger_value, trigger_condition and price are required. MANUAL - The order is inactive in the Nordnet system until it is manually activated by the customer. OCO_STOP_ACTPRICE - One cancels other orders. Our implementation is one normal order and one stop-loss order. If the real order is executed the stop-loss is cancelled. If the stop-loss triggers the normal order is canceled. This combination is always displayed as one order.
- **currency optional** (string) — The currency that the instrument is traded in.
- **extended_hours optional** (boolean) — If true, order is applicable for US pre-market trading. Defaults to false.
- **identifier optional** (string) — Nordnet tradable identifier.
- **market_id required** (integer(int64)) — Nordnet market identifier.
- **open_volume optional** (integer(int64)) — The visible part of an iceberg order. If this field is omitted the whole volume of the order is visible on the market. This field is only allowed if the order type is LIMIT or NORMAL.
- **order_type optional** (enum (FAK, FOK, NORMAL, LIMIT, STOP_LIMIT, STOP_TRAILING, OCO)) — The order type. Defaults to NORMAL. When using NORMAL the system guesses the order type based on used parameters. For better parameter validation and to ensure that the order type is the desired one the client should not use NORMAL, but rather the intended order type instead. NORMAL will be deprecated in future versions.
- **price optional** (number(double)) — The price limit of the order.
- **reference optional** (string) — Free text reference for the order. Intended for the customer.
- **side required** (enum (BUY, SELL)) — BUY or SELL.
- **target_value optional** (number(double)) — Only used when type is STOP_ACTPRICE_PERC or OCO_STOP_ACTPRICE. This is the price on the market. If type is STOP_ACTPRICE_PERC the value is given in percentage points. The price will be trailing_value + (target_value percentage of trailing_value). If type is OCO_STOP_ACTPRICE the price is a fixed price.
- **trigger_condition optional** (string) — The comparison that should be used on trigger_value. Valid values are <= (less than or equal to) or >= (greater than or equal to).
- **trigger_value optional** (number(double)) — If type is STOP_ACTPRICE_PERC the value is given in percentage points. Minimum value is 1 for STOP_ACTPRICE_PERC. If type is STOP_ACTPRICE the value is a fixed price.
- **valid_until optional** (string(date)) — Date formatted as YYYY-MM-DD. If this field is left out the order is a day order - that is, the same behavior as if valid_until is set to today. Smart-orders can only be day orders.
- **volume required** (integer(int64)) — The volume of the order.

## Response Body Schema

- **200**: OrderReply

## Status Codes

| HTTP Code | Description | Schema |
|-----------|-------------|--------|
| 200 | Standard response for successful HTTP requests. | OrderReply |
| 400 | Invalid parameter. | ErrorResponse |
| 401 | Invalid session. | ErrorResponse |
| 403 | User is logged in but user or system does not have privileges to use this endpoint. | No Content |
| 429 | Too Many Requests. Please wait for 10 seconds before retrying. | ErrorResponse |
| 503 | Service Unavailable. Follow the Retry-After header and retry in the specified amount of seconds. | ErrorResponse |

## Examples

_(no example blocks in documentation HTML)_

## Doc inconsistencies

_(none identified during Phase 1 extraction)_
