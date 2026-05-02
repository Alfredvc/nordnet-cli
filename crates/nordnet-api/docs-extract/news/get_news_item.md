# get_news_item — Retrieve news

## Endpoint

`GET /api/2/news/{item_id}`

## Description

Retrieve one or more news articles.

## Parameters

| Type | Name | Description | Schema |
|------|------|-------------|--------|
| Header | Accept-Language optional | Language preferred in the response. Overrides the session language. Note that nb and nn are equivalent to no. | enum (da, de, en, fi, nb, nn, no, sv) |
| Header | Authorization optional | Basic authorization header, that is the value must be Basic <credentials> where <credentials> is the base64 encoded string <session_id>:<session_id>. | string |
| Path | item_id required | Article ID. Multiple article IDs must be comma-separated. | < integer(int64) > array |
| Query | body_format optional | Deprecated parameter. Has no effect on the response. | enum (plain, markdown) |

## Request Body Schema

_(none)_

## Response Body Schema

- **200**: < NewsArticle > array
- **204**: No Content

## Status Codes

| HTTP Code | Description | Schema |
|-----------|-------------|--------|
| 200 | Standard response for successful HTTP requests. | < NewsArticle > array |
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
- [NewsArticle](../_definitions/NewsArticle.md)
