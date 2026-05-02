# ApiKeyVerifyLoginRequest

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| api_key | required | The API key provided by Nordnet. The API key can always be found on your profile page after uploading your public key. | string |
| service | required | The service name (provided by Nordnet). | string |
| signature | required | The signed and base64 encoded challenge string created by the user. | string |
