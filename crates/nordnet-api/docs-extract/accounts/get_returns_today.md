# get_returns_today — Get today’s withdrawal/deposit transaction amounts

## Endpoint

`GET /api/2/accounts/{accid}/returns/transactions/today`

## Description

Returns today’s withdrawal/deposit transaction amounts for the given account.

## Parameters

| Type | Name | Description | Schema | Default |
|------|------|-------------|--------|----------|
| Header | Accept-Language optional | Language preferred in the response. Overrides the session language. Note that nb and nn are equivalent to no. | enum (da, de, en, fi, nb, nn, no, sv) |  |
| Header | Authorization optional | Basic authorization header, that is the value must be Basic <credentials> where <credentials> is the base64 encoded string <session_id>:<session_id>. | string |  |
| Path | accid required | The account identifier for the account. Some systems can or must use the account number as account identifier. | integer(int64) |  |
| Query | include_credit_account optional | true if credit accounts should be included in the response. Defaults to true. | boolean | "true" |

## Request Body Schema

_(none)_

## Response Body Schema

- **200**: < AccountTransactionsToday > array
- **204**: No Content

## Status Codes

| HTTP Code | Description | Schema |
|-----------|-------------|--------|
| 200 | Standard response for successful HTTP requests. | < AccountTransactionsToday > array |
| 204 | No content. | No Content |
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

- [AccountTransactionsToday](../_definitions/AccountTransactionsToday.md)
- [ErrorResponse](../_definitions/ErrorResponse.md)
