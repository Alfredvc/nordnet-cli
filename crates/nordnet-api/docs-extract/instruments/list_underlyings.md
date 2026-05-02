# list_underlyings — Get underlyings

## Endpoint

`GET /api/2/instruments/underlyings/{derivative_type}/{currency}`

## Description

Returns instruments that are underlyings for a specific type of instruments. Can return instruments that have option derivatives or leverage derivatives. Warrants are included in the leverage derivatives.

## Parameters

| Type | Name | Description | Schema |
|------|------|-------------|--------|
| Header | Accept-Language optional | Language preferred in the response. Overrides the session language. Note that nb and nn are equivalent to no. | enum (da, de, en, fi, nb, nn, no, sv) |
| Header | Authorization optional | Basic authorization header, that is the value must be Basic <credentials> where <credentials> is the base64 encoded string <session_id>:<session_id>. | string |
| Path | currency required | The derivative currency. Please note that the underlying can have a different currency. | string |
| Path | derivative_type required | Specifies which instrument type to find underlyings for. | enum (leverage, option_pair) |

## Request Body Schema

_(none)_

## Response Body Schema

- **200**: < Instrument > array
- **204**: No Content

## Status Codes

| HTTP Code | Description | Schema |
|-----------|-------------|--------|
| 200 | Standard response for successful HTTP requests. | < Instrument > array |
| 204 | No content. | No Content |
| 400 | Invalid parameter. | ErrorResponse |
| 401 | Invalid session. | ErrorResponse |
| 429 | Too Many Requests. Please wait for 10 seconds before retrying. | ErrorResponse |
| 503 | Service Unavailable. Follow the Retry-After header and retry in the specified amount of seconds. | ErrorResponse |

## Examples

_(no example blocks in documentation HTML)_

## Doc inconsistencies

_(none identified during Phase 1 extraction)_
