# search — Search Nordnet

## Endpoint

`GET /api/2/main_search`

## Description

Returns the instruments, news, and pages matching the given search criteria.

## Parameters

| Type | Name | Description | Schema | Default |
|------|------|-------------|--------|----------|
| Header | Accept-Language optional | Language preferred in the response. Overrides the session language. Note that nb and nn are equivalent to no. | enum (da, de, en, fi, nb, nn, no, sv) |  |
| Header | Authorization optional | Basic authorization header, that is the value must be Basic <credentials> where <credentials> is the base64 encoded string <session_id>:<session_id>. | string |  |
| Query | instrument_group optional | Defines the instrument groups to be searched. When not specified the search returns results from all instrument groups. | < enum (EQUITY, PINV, FUND, ETF, ETC, WARRANT, DERIVATIVES, INDICATOR, OTHER) > array |  |
| Query | limit optional | Limits the search results per group to limit. The default limit is 5. Combine with offset to implement pagination. | integer(int32) | "5" |
| Query | offset optional | Skips the first offset search results per group. The default offset is 0. Combine with limit to implement pagination. | integer(int32) | "0" |
| Query | query required | Search string. | string |  |
| Query | search_space optional | Search space. | enum (ALL, INSTRUMENTS, NEWS, CMS, BLOG, INSTRUMENTS_NEWS, INSTRUMENTS_CMS, NEWS_CMS, NEWS_BLOG, NEWS_BLOG_CMS) | "ALL" |

## Request Body Schema

_(none)_

## Response Body Schema

- **200**: < MainSearchResponse > array
- **204**: No Content

## Status Codes

| HTTP Code | Description | Schema |
|-----------|-------------|--------|
| 200 | Standard response for successful HTTP requests. | < MainSearchResponse > array |
| 204 | No content. | No Content |
| 400 | Invalid parameter. | ErrorResponse |
| 401 | Invalid session. | ErrorResponse |
| 404 | Not found. | No Content |
| 429 | Too Many Requests. Please wait for 10 seconds before retrying. | ErrorResponse |
| 503 | Service Unavailable. Follow the Retry-After header and retry in the specified amount of seconds. | ErrorResponse |

## Examples

_(no example blocks in documentation HTML)_

## Doc inconsistencies

_(none identified during Phase 1 extraction)_

## Referenced types

- [ErrorResponse](../_definitions/ErrorResponse.md)
- [MainSearchResponse](../_definitions/MainSearchResponse.md)
