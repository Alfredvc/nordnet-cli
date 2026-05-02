# Trade

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| accid | optional | The account identifier. An account identifier refers to a specific account for the duration of a session, but this is not guaranteed between sessions. Not applicable for partners. | integer(int64) |
| accno | required | The Nordnet account number. An account number always refers to a specific account. | integer(int64) |
| counterparty | optional | The counterparty if available. | string |
| order_id | required | Nordnet order identifier. | integer(int64) |
| price | required | The price of the trade. | Amount |
| side | required | BUY or SELL. | string |
| tradable | required | The tradable identifier. | TradableId |
| trade_id | optional | Trade identifier from the market if available. | string |
| tradetime | required | The time of the trade. UNIX timestamp in milliseconds. | integer(int64) |
| volume | required | The volume of the trade. | number(double) |
