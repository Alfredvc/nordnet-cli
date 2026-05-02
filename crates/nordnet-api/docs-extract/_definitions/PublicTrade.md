# PublicTrade

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| broker_buying | optional | Buying participant. | string |
| broker_selling | optional | Selling participant. | string |
| market_id | required | Market ID | integer(int64) |
| price | required | The price of the trade. | number(double) |
| tick_timestamp | required | Tick timestamp. Unix time in milliseconds. | integer(int64) |
| trade_id | required | The trade ID on the exchange. | string |
| trade_timestamp | required | Trade timestamp. Unix time in milliseconds. | integer(int64) |
| trade_type | optional | The trade type defined by the exchange. | string |
| volume | required | The volume of the trade. | integer(int64) |
