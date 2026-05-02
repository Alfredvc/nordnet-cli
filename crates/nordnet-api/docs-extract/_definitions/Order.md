# Order

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| accid | optional | The account identifier. An account identifier refers to a specific account for the duration of a session, but this is not guaranteed between sessions. Not applicable for partners. | integer(int64) |
| accno | required | The Nordnet account number. An account number always refers to a specific account. | integer(int64) |
| action_state | required | The state of the last action performed on the order: DEL_FAIL - Delete request failed and the order is still active on the market. DEL_PEND - Delete request in progress and unconfirmed by the market. DEL_CONF - Delete confirmed by the market. DEL_PUSH - Deleted by the market. INS_FAIL - Insert failed. INS_PEND - Pending insert. INS_CONF - Confirmed insert. INS_STOP - The order inserted into the Nordnet system and stopped. This is the state of inactive orders and not triggered stop-loss orders. MOD_FAIL - Modification failed and the previous order values are still valid. MOD_PEND - Modification in progress and waiting confirmation from the market. MOD_PUSH - Modified by the market. INS_WAIT - Insert waiting for market opening. MOD_WAIT - Modification of order on the market, waiting for market opening. DEL_WAIT - Delete of order on the market, waiting for market opening. MOD_CONF - Modification confirmed by the market. | string |
| activation_condition | optional | The activation condition for stop-loss orders. | ActivationCondition |
| modified | required | Last modification time of the order. UNIX timestamp in milliseconds. | integer(int64) |
| open_volume | optional | The open volume of an iceberg order. | number(double) |
| order_id | required | The Nordnet order identifier. | integer(int64) |
| order_state | required | The state of the order: DELETED - Order is deleted. LOCAL - The order is offline/local and eligible for activation. ON_MARKET - The order is active on the market. LOCKED - The order can’t be modified by the customer. | string |
| order_type | required | The type of the order. Each order type assumes a certain combination of parameters, for instance fill-and-kill requires certain validity and volume conditions. These predefined combinations of parameters can also be used for input validation. | string |
| price | required | The price of the order. | Amount |
| price_condition | required | The price condition on the order: LIMIT - The order is limited by the given price. AT_MARKET - The order is entered at the current market price. This is not supported by most markets. | string |
| reference | optional | Customer reference for the order. This is a free text field for the customer. | string |
| side | required | BUY or SELL. | string |
| tradable | required | The tradable identifier. | TradableId |
| traded_volume | required | The total traded volume of the order. | number(double) |
| validity | required | The validity period for the order. | Validity |
| volume | required | The original volume of the order. | number(double) |
| volume_condition | required | The volume condition on the order: NORMAL - All types of fills are accepted. ALL_OR_NOTHING - Partial fills are not accepted. | string |
