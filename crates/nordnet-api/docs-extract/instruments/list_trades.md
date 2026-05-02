# list_trades — Get todays public trades

## Endpoint

`GET /api/2/instruments/{instrument_id}/trades`

## Description

Returns all the public trades (trades executed on the marketplace) belonging to one or more instruments.

## Parameters

| Type | Name | Description | Schema |
|------|------|-------------|--------|
| Header | Accept-Language optional | Language preferred in the response. Overrides the session language. Note that nb and nn are equivalent to no. | enum (da, de, en, fi, nb, nn, no, sv) |
| Header | Authorization optional | Basic authorization header, that is the value must be Basic <credentials> where <credentials> is the base64 encoded string <session_id>:<session_id>. | string |
| Path | instrument_id required | The instrument identifier. One or more instrument identifiers can be specified. Multiple inputs must be comma separated. | < integer(int64) > array |
| Query | count optional | The number of trades to return. Integer value or all. Defaults to 5. | string |

## Request Body Schema

_(none)_

## Response Body Schema

- **200**: < InstrumentPublicTrades > array
- **204**: No Content

## Status Codes

| HTTP Code | Description | Schema |
|-----------|-------------|--------|
| 200 | Standard response for successful HTTP requests. | < InstrumentPublicTrades > array |
| 204 | No content. | No Content |
| 400 | Invalid parameter. | ErrorResponse |
| 401 | Invalid session. | ErrorResponse |
| 429 | Too Many Requests. Please wait for 10 seconds before retrying. | ErrorResponse |
| 503 | Service Unavailable. Follow the Retry-After header and retry in the specified amount of seconds. | ErrorResponse |

## Examples

_(no example blocks in documentation HTML)_

## Doc inconsistencies

_(none identified during Phase 1 extraction)_
