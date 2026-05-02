# search_optionlist_pairs — Search for the Option Pair (Put-Call) given an underlying instrument and the expiration date.

## Endpoint

`GET /api/2/instrument_search/query/optionlist/pairs`

## Description

Intended for displaying Option Pair lists to the end user grouped by underlying instrument and expiration date. Access to real-time prices is handled based on access rights. When real-time prices are returned, real-time snapshot access logs are produced.

## Parameters

| Type | Name | Description | Schema |
|------|------|-------------|--------|
| Header | Accept-Language optional | Language preferred in the response. Overrides the session language. Note that nb and nn are equivalent to no. | enum (da, de, en, fi, nb, nn, no, sv) |
| Header | Authorization optional | Basic authorization header, that is the value must be Basic <credentials> where <credentials> is the base64 encoded string <session_id>:<session_id>. | string |
| Query | currency required | Search for options with the given currency. | string |
| Query | expire_date required | Search for options with the given expiration date. | integer(int64) |
| Query | underlying_symbol required | Search for options with the given underlying instrument symbol. | string |

## Request Body Schema

_(none)_

## Response Body Schema

- **200**: OptionListResults

## Status Codes

| HTTP Code | Description | Schema |
|-----------|-------------|--------|
| 200 | Standard response for successful HTTP requests. | OptionListResults |
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
- [OptionListResults](../_definitions/OptionListResults.md)
