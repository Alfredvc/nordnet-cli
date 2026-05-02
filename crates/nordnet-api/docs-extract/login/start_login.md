# start_login — Start authentication challenge

## Endpoint

`POST /api/2/login/start`

## Description

This endpoint initiates the authentication process by generating a challenge string that you must sign with your private key for the next step in the login process.Before you start using this authentication method, you need to generate an SSH key pair and add the public key to your user settings on the Nordnet web platform.A detailed description of all required steps for a complete login can be found in the Getting started guide.

## Parameters

| Type | Name | Description | Schema |
|------|------|-------------|--------|
| Header | Accept-Language optional | Language preferred in the response. Overrides the session language. Note that nb and nn are equivalent to no. | enum (da, de, en, fi, nb, nn, no, sv) |
| Body | body required | API key start login request | ApiKeyStartLoginRequest |

## Request Body Schema

_(form data parameters)_

- **body required** (ApiKeyStartLoginRequest) — API key start login request

## Response Body Schema

- **200**: ChallengeResponse

## Status Codes

| HTTP Code | Description | Schema |
|-----------|-------------|--------|
| 200 | Standard response for successful HTTP requests. | ChallengeResponse |
| 400 | Invalid parameter. | ErrorResponse |
| 429 | Too Many Requests. Please wait for 10 seconds before retrying. | ErrorResponse |
| 503 | Service Unavailable. Follow the Retry-After header and retry in the specified amount of seconds. | ErrorResponse |

## Examples

_(no example blocks in documentation HTML)_

## Doc inconsistencies

_(none identified during Phase 1 extraction)_

## Referenced types

- [ApiKeyStartLoginRequest](../_definitions/ApiKeyStartLoginRequest.md)
- [ChallengeResponse](../_definitions/ChallengeResponse.md)
- [ErrorResponse](../_definitions/ErrorResponse.md)
