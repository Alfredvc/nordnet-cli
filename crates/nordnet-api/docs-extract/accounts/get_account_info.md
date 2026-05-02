# get_account_info — Get account information summary

## Endpoint

`GET /api/2/accounts/{accid}/info`

## Description

Returns account information details for one or more account.

## Parameters

| Type | Name | Description | Schema |
|------|------|-------------|--------|
| Header | Accept-Language optional | Language preferred in the response. Overrides the session language. Note that nb and nn are equivalent to no. | enum (da, de, en, fi, nb, nn, no, sv) |
| Header | Authorization optional | Basic authorization header, that is the value must be Basic <credentials> where <credentials> is the base64 encoded string <session_id>:<session_id>. | string |
| Path | accid required | One or more account identifier that identifies the accounts. Some systems can or must use the account number as account identifier. Multiple account identifiers must be comma separated. | < integer > array |
| Query | include_interest_rate optional | true if interest_rate should be included in the response. Note that performance improves when the flag is set to false. Defaults to true. | boolean |
| Query | include_short_pos_margin optional | true if short_postion_margin should be included in the response. Defaults to true. | boolean |

## Request Body Schema

_(none)_

## Response Body Schema

- **200**: < AccountInfo > array

## Status Codes

| HTTP Code | Description | Schema |
|-----------|-------------|--------|
| 200 | Standard response for successful HTTP requests. | < AccountInfo > array |
| 400 | Invalid parameter. | ErrorResponse |
| 401 | Invalid session. | ErrorResponse |
| 403 | User is logged in but user or system does not have privileges to use this endpoint. | No Content |
| 429 | Too Many Requests. Please wait for 10 seconds before retrying. | ErrorResponse |
| 503 | Service Unavailable. Follow the Retry-After header and retry in the specified amount of seconds. | ErrorResponse |

## Examples

_(no example blocks in documentation HTML)_

## Doc inconsistencies

_(none identified during Phase 1 extraction)_

## Referenced types

- [AccountInfo](../_definitions/AccountInfo.md)
- [ErrorResponse](../_definitions/ErrorResponse.md)
