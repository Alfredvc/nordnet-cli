# lookup — Instrument lookup

## Endpoint

`GET /api/2/instruments/lookup/{lookup_type}/{lookup}`

## Description

Lookup specfic instrument with predefined fields. Please note that this is not a search, only exact matches are returned.

## Parameters

| Type | Name | Description | Schema |
|------|------|-------------|--------|
| Header | Accept-Language optional | Language preferred in the response. Overrides the session language. Note that nb and nn are equivalent to no. | enum (da, de, en, fi, nb, nn, no, sv) |
| Header | Authorization optional | Basic authorization header, that is the value must be Basic <credentials> where <credentials> is the base64 encoded string <session_id>:<session_id>. | string |
| Path | lookup required | If the lookup type is market_id_identifier the lookup must be formatted as [market_id]:[identifier]. If the lookup type is isin_code_currency_market_id the lookup must be formatted as [isin]:[currency]:[market_id]. Multiple entries must be comma separated. | < string > array |
| Path | lookup_type required | The lookup type to use. | enum (market_id_identifier, isin_code_currency_market_id) |

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

## Referenced types

- [ErrorResponse](../_definitions/ErrorResponse.md)
- [Instrument](../_definitions/Instrument.md)
