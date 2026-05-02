# ActivationCondition

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| trailing_value | optional | The fix point that the trigger_value and target_value percent is calculated from. Only used when type is STOP_ACTPRICE_PERC. | number(double) |
| trigger_condition | optional | The comparison that should be used on trigger_value. Valid values are <= (less than or equal to) or >= (greater than or equal to). | string |
| trigger_value | optional | The trigger value. If type is STOP_ACTPRICE_PERC the value is given in percentage points. If type is STOP_ACTPRICE the value is a fixed price. | number(double) |
| type | required | The stop-loss activation condition: NONE - This order has no activation condition. It is sent directly to the market (if the market is open). MANUAL - The order is inactive in the Nordnet system and is activated by the customer. STOP_ACTPRICE_PERC - Trailing stop-loss. The order is activated when the price changes by the given percentage. STOP_ACTPRICE - The order is activated when the market price of the instrument reaches a trigger price. | string |
