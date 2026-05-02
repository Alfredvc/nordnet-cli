# verify_login — Complete session (log in)

## Endpoint

`POST /api/2/login/verify`

## Description

This endpoint completes the login process.A detailed description of all required steps for a complete login can be found in the Getting started guide.

## Parameters

| Type | Name | Description | Schema |
|------|------|-------------|--------|
| Header | Accept-Language optional | Language preferred in the response. Overrides the session language. Note that nb and nn are equivalent to no. | enum (da, de, en, fi, nb, nn, no, sv) |
| Body | body required | API key verify login request | ApiKeyVerifyLoginRequest |

## Request Body Schema

_(form data parameters)_

- **body required** (ApiKeyVerifyLoginRequest) — API key verify login request

## Response Body Schema

- **200**: ApiKeyLoginResponse

## Status Codes

| HTTP Code | Description | Schema |
|-----------|-------------|--------|
| 200 | Standard response for successful HTTP requests. | ApiKeyLoginResponse |
| 400 | Invalid parameter. | ErrorResponse |
| 401 | Unauthorized to log in using the given credentials. | ErrorResponse |
| 429 | Too Many Requests. Please wait for 10 seconds before retrying. | ErrorResponse |
| 503 | Service Unavailable. Follow the Retry-After header and retry in the specified amount of seconds. | ErrorResponse |

## Examples

_(no example blocks in documentation HTML)_

## Doc inconsistencies

_(none identified during Phase 1 extraction)_

## Referenced types

- [ApiKeyLoginResponse](../_definitions/ApiKeyLoginResponse.md)
- [ApiKeyVerifyLoginRequest](../_definitions/ApiKeyVerifyLoginRequest.md)
- [ErrorResponse](../_definitions/ErrorResponse.md)
