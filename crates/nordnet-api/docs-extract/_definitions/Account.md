# Account

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| accid | optional | The account identifier. An account identifier refers to a specific account for the duration of a session, but this is not guaranteed between sessions. Not applicable for partners. | integer(int64) |
| accno | required | The Nordnet account number. An account number always refers to a specific account. | integer(int64) |
| alias | required | The account alias. This can be set by the customer. | string |
| atyid | optional | The account type identifier | integer(int32) |
| blocked_reason | optional | The reason why the account is blocked. This field is translated to the language specified in the request. | string |
| default | required | true if this is the default account. | boolean |
| is_blocked | optional | true if the account is blocked. No queries can be made against a blocked account. | boolean |
| type | required | The account type. This field is translated. | string |
