# Position

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| accid | optional | The account identifier. An account identifier refers to a specific account for the duration of a session, but this is not guaranteed between sessions. Not applicable for partners. | integer(int64) |
| accno | required | The Nordnet account number. An account number always refers to a specific account. | integer(int64) |
| acq_price | required | The acquisition price in the tradable currency. | Amount |
| acq_price_acc | required | The acquisition price in the account currency. | Amount |
| instrument | required | The position instrument. | Instrument |
| margin_percent | required | The collateral percentage required to cover this position if short (qty < 0). | integer(int32) |
| market_value | required | The market value in the tradable currency. | Amount |
| market_value_acc | required | The market value in the account currency. | Amount |
| morning_price | required | The price of the position instrument in the morning | Amount |
| pawn_percent | required | The percentage the user is allowed loan on this position. | integer(int32) |
| qty | required | The quantity of the position. | number(float) |
