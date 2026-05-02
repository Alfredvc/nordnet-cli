# search_unlimitedturbolist — Search, filter and sort instruments within the Unlimited Turbo entity type.

## Endpoint

`GET /api/2/instrument_search/query/unlimitedturbolist`

## Description

Intended for displaying Unlimited Turbo instruments to the end user, grouped by lists. To return only "Nordnet markets" instruments use the apply_filter parameter nordnet_markets=true. Access to real-time prices is handled based on access rights. When real-time prices are returned, real-time snapshot access logs are produced.

## Parameters

| Type | Name | Description | Schema | Default |
|------|------|-------------|--------|----------|
| Header | Accept-Language optional | Language preferred in the response. Overrides the session language. Note that nb and nn are equivalent to no. | enum (da, de, en, fi, nb, nn, no, sv) |  |
| Header | Authorization optional | Basic authorization header, that is the value must be Basic <credentials> where <credentials> is the base64 encoded string <session_id>:<session_id>. | string |  |
| Query | apply_filters optional | Specifies which filters to apply to the search. | string |  |
| Query | free_text_search optional | Free text search for name, symbol and ISIN. | string |  |
| Query | limit optional | Limits the search results to limit instruments. The default limit is 50. Combine with offset to implement pagination. | integer(int32) | "50" |
| Query | offset optional | Skips the first offset search results. The default offset is 0. Combine with limit to implement pagination. | integer(int32) | "0" |
| Query | sort_attribute optional | Defines the attribute to sort the search results by. | string |  |
| Query | sort_order optional | Defines the sort order of the search results. Use asc for ascending order and desc for descending order. | enum (asc, desc) |  |

## Request Body Schema

_(none)_

## Response Body Schema

- **200**: UnlimitedTurboListResults

## Status Codes

| HTTP Code | Description | Schema |
|-----------|-------------|--------|
| 200 | Standard response for successful HTTP requests. | UnlimitedTurboListResults |
| 400 | Invalid parameter. | ErrorResponse |
| 401 | Invalid session. | ErrorResponse |
| 429 | Too Many Requests. Please wait for 10 seconds before retrying. | ErrorResponse |
| 503 | Service Unavailable. Follow the Retry-After header and retry in the specified amount of seconds. | ErrorResponse |

## Examples

_(no example blocks in documentation HTML)_

## Doc inconsistencies

_(none identified during Phase 1 extraction)_
