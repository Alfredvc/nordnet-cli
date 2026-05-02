# list_accounts — Get user accounts

## Endpoint

`GET /api/2/accounts`

## Description

Returns a list of accounts to which the user has access.

## Parameters

| Type | Name | Description | Schema |
|------|------|-------------|--------|
| Header | Accept-Language optional | Language preferred in the response. Overrides the session language. Note that nb and nn are equivalent to no. | enum (da, de, en, fi, nb, nn, no, sv) |
| Header | Authorization optional | Basic authorization header, that is the value must be Basic <credentials> where <credentials> is the base64 encoded string <session_id>:<session_id>. | string |
| Query | include_credit_accounts optional | true if credit accounts should be included in the response. Defaults to false. | boolean |

## Request Body Schema

_(none)_

## Response Body Schema

- **200**: < Account > array
- **204**: No Content

## Status Codes

| HTTP Code | Description | Schema |
|-----------|-------------|--------|
| 200 | Standard response for successful HTTP requests. | < Account > array |
| 204 | No content. | No Content |
| 401 | Invalid session. | ErrorResponse |
| 403 | User is logged in but user or system does not have privileges to use this endpoint. | No Content |
| 429 | Too Many Requests. Please wait for 10 seconds before retrying. | ErrorResponse |
| 503 | Service Unavailable. Follow the Retry-After header and retry in the specified amount of seconds. | ErrorResponse |

## Examples

_(no example blocks in documentation HTML)_

## Doc inconsistencies

_(none identified during Phase 1 extraction)_
