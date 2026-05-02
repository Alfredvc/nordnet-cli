# get_leverage_filters — Get leverage instruments filters

## Endpoint

`GET /api/2/instruments/{instrument_id}/leverages/filters`

## Description

Returns valid leverage instruments filter values. Can be used to fill combo boxes in client applications in order to filter leverage instruments results. The same filters can be applied on this request to exclude invalid filter combinations.

## Parameters

| Type | Name | Description | Schema |
|------|------|-------------|--------|
| Header | Accept-Language optional | Language preferred in the response. Overrides the session language. Note that nb and nn are equivalent to no. | enum (da, de, en, fi, nb, nn, no, sv) |
| Header | Authorization optional | Basic authorization header, that is the value must be Basic <credentials> where <credentials> is the base64 encoded string <session_id>:<session_id>. | string |
| Path | instrument_id required | The underlying instrument ID. | integer(int64) |
| Query | currency optional | Show only leverage instruments with a specific currency. | string |
| Query | expiration_date optional | Show only leverage instruments with a specific expiration date. | string(date) |
| Query | instrument_group_type optional | Show only instruments with a specific instrument group type. | string |
| Query | instrument_type optional | Show only instruments with a specific instrument type. | string |
| Query | issuer_id optional | Show only leverage instruments from a specific issuer. | integer(int64) |
| Query | market_view optional | Show only leverage instruments with a specific market view. | enum (D, U) |

## Request Body Schema

_(none)_

## Response Body Schema

- **200**: LeverageFilter

## Status Codes

| HTTP Code | Description | Schema |
|-----------|-------------|--------|
| 200 | Standard response for successful HTTP requests. | LeverageFilter |
| 400 | Invalid parameter. | ErrorResponse |
| 401 | Invalid session. | ErrorResponse |
| 429 | Too Many Requests. Please wait for 10 seconds before retrying. | ErrorResponse |
| 503 | Service Unavailable. Follow the Retry-After header and retry in the specified amount of seconds. | ErrorResponse |

## Examples

_(no example blocks in documentation HTML)_

## Doc inconsistencies

_(none identified during Phase 1 extraction)_
