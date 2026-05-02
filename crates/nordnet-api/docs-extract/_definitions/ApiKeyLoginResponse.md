# ApiKeyLoginResponse

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| expires_in | required | The session expiration interval in seconds. Note that this is not the remaining time until session time-out but rather the entire interval. | integer(int64) |
| private_feed | required | Connection information for the Private Feed. | Feed |
| public_feed | required | Connection information for the Public Feed. | Feed |
| session_key | required | The session key used for identification in all other requests. | string |
