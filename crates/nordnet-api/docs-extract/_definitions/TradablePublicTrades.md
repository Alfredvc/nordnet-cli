# TradablePublicTrades

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| identifier | required | The tradable identifier. The combination of market ID and tradable identifier is unique. | string |
| market_id | required | The Nordnet unique market identifier. | integer(int64) |
| trades | required | A list of the public trades. | < PublicTrade > array |
