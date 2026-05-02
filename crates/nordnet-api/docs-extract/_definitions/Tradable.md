# Tradable

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| display_order | required | Determine the display order of the tradables for an instrument. The tradables should be shown in the order of increasing display_order numbers. | integer(int64) |
| identifier | required | Nordnet tradable identifier. The combination of market ID and identifier is unique. | string |
| lot_size | required | The lot size of the tradable. | number(double) |
| market_id | required | Nordnet market identifier. | integer(int64) |
| mic | required | The market identifier code (MIC) of the tradable. | string |
| price_unit | required | The unit that the prices is sent in. Examples: GBX for GBP with multiplier 0.01, % for bonds and the same as currency for normal instruments. | string |
| tick_size_id | required | Tick size identifier. | integer(int64) |
