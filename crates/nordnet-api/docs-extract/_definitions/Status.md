# Status

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| message | required | Additional information. | string |
| system_running | required | Indicates if the system is running or temporarily stopped. | boolean |
| timestamp | required | Server time. UNIX timestamp in milliseconds. | integer(int64) |
| valid_version | required | true if the API version is valid. | boolean |
