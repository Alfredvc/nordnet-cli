# TradableInfo

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| calendar | required | Allowed days for long term orders. | < CalendarDay > array |
| iceberg | required | true if iceberg orders are allowed. | boolean |
| identifier | required | The Nordnet tradable identifier. The combination of market ID and tradable ID is unique. | string |
| market_id | required | The Nordnet unique market identifier. | integer(int64) |
| order_types | required | Allowed order types. | < OrderType > array |
