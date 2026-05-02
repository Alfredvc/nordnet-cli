# PriceInfo

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| ask | optional | Ask price, top of book. | PriceWithDecimals |
| ask_volume | optional | Ask volume, top of book. | integer(int64) |
| bid | optional | Bid price, top of book. | PriceWithDecimals |
| bid_volume | optional | Bid volume, top of book. | integer(int64) |
| close | optional | Close price. | PriceWithDecimals |
| diff | optional | Price difference since the last close price. | DiffWithDecimals |
| diff_pct | optional | Percent difference since the last close price. | number(double) |
| high | optional | Highest paid today. | PriceWithDecimals |
| last | optional | Last price. | PriceWithDecimals |
| low | optional | Lowest paid today. | PriceWithDecimals |
| open | optional | Open price. | PriceWithDecimals |
| realtime | optional | Set to true if the price information is based on a real-time snapshot. | boolean |
| spread | optional | Bid-ask spread. | PriceWithDecimals |
| spread_pct | optional | Bid-ask spread percent. | number(double) |
| tick_timestamp | optional | Last tick time stamp. | integer(int64) |
| turnover | optional | Daily turnover. | number(double) |
| turnover_volume | optional | Turnover volume. | integer(int64) |
