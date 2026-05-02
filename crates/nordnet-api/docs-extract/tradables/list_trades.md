# list_trades — Get todays public trades

## Endpoint

`GET /api/2/tradables/trades/{tradables}`

## Description

Returns a list of public trades (all trades executed on the marketplace) for the given tradable(s).

## Parameters

| Type | Name | Description | Schema |
|------|------|-------------|--------|
| Header | Accept-Language optional | Language preferred in the response. Overrides the session language. Note that nb and nn are equivalent to no. | enum (da, de, en, fi, nb, nn, no, sv) |
| Header | Authorization optional | Basic authorization header, that is the value must be Basic <credentials> where <credentials> is the base64 encoded string <session_id>:<session_id>. | string |
| Path | tradables required | [market_id]:[identifier] of the tradable. Example: 11:101 for ERIC B. Multiple inputs must be comma separated. | < string > array |
| Query | count optional | Number of trades to return. Integer value or all. Defaults to 5. | string |

## Request Body Schema

_(none)_

## Response Body Schema

- **200**: < TradablePublicTrades > array
- **204**: No Content

## Status Codes

| HTTP Code | Description | Schema |
|-----------|-------------|--------|
| 200 | Standard response for successful HTTP requests. | < TradablePublicTrades > array |
| 204 | No content. | No Content |
| 400 | Invalid parameter. | ErrorResponse |
| 401 | Invalid session. | ErrorResponse |
| 429 | Too Many Requests. Please wait for 10 seconds before retrying. | ErrorResponse |
| 503 | Service Unavailable. Follow the Retry-After header and retry in the specified amount of seconds. | ErrorResponse |

## Examples

_(no example blocks in documentation HTML)_

## Doc inconsistencies

_(none identified during Phase 1 extraction)_
