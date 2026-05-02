# modify_order — Modify an order

## Endpoint

`PUT /api/2/accounts/{accid}/orders/{order_id}`

## Description

Modifies the price and/or volume of an order.

## Parameters

| Type | Name | Description | Schema |
|------|------|-------------|--------|
| Header | Accept-Language optional | Language preferred in the response. Overrides the session language. Note that nb and nn are equivalent to no. | enum (da, de, en, fi, nb, nn, no, sv) |
| Header | Authorization optional | Basic authorization header, that is the value must be Basic <credentials> where <credentials> is the base64 encoded string <session_id>:<session_id>. | string |
| Path | accid required | The account identifier for the account. Some systems can or must use the account number as account identifier. | integer(int64) |
| Path | order_id required | The identifier of the order to modify. | integer(int64) |
| FormData | currency optional | The currency of the instrument. Required when the price is changed. | string |
| FormData | open_volume optional | The new open volume. | integer(int64) |
| FormData | price optional | The new price. If left out the price is left unchanged. | number(double) |
| FormData | volume optional | The new volume. | integer(int64) |

## Request Body Schema

_(form data parameters)_

- **currency optional** (string) — The currency of the instrument. Required when the price is changed.
- **open_volume optional** (integer(int64)) — The new open volume.
- **price optional** (number(double)) — The new price. If left out the price is left unchanged.
- **volume optional** (integer(int64)) — The new volume.

## Response Body Schema

- **200**: OrderReply

## Status Codes

| HTTP Code | Description | Schema |
|-----------|-------------|--------|
| 200 | Standard response for successful HTTP requests. | OrderReply |
| 400 | Invalid parameter. | ErrorResponse |
| 401 | Invalid session. | ErrorResponse |
| 403 | User is logged in but user or system does not have privileges to use this endpoint. | No Content |
| 404 | Order not found. | ErrorResponse |
| 429 | Too Many Requests. Please wait for 10 seconds before retrying. | ErrorResponse |
| 503 | Service Unavailable. Follow the Retry-After header and retry in the specified amount of seconds. | ErrorResponse |

## Examples

_(no example blocks in documentation HTML)_

## Doc inconsistencies

_(none identified during Phase 1 extraction)_

## Referenced types

- [ErrorResponse](../_definitions/ErrorResponse.md)
- [Order](../_definitions/Order.md)
- [OrderReply](../_definitions/OrderReply.md)
