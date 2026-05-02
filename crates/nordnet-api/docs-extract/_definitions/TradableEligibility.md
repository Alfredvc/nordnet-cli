# TradableEligibility

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| eligible | required | true if the customer is eligible to trade the tradable. | boolean |
| identifier | required | The tradable identifier. The combination of market ID and tradable ID is unique. | string |
| market_id | required | The market identifier. | integer(int32) |
