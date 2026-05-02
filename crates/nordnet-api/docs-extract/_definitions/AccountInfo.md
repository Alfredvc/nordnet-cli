# AccountInfo

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| accid | optional | The account identifier. An account identifier refers to a specific account for the duration of a session, but this is not guaranteed between sessions. Not applicable for partners. | integer(int64) |
| accno | required | The Nordnet account number. An account number always refers to a specific account. | integer(int64) |
| account_credit | required | The account credit. | Amount |
| account_currency | required | The account currency. | string |
| account_sum | required | The combined sum of all ledgers. | Amount |
| bonus_cash | optional | The bonus cash if available. | Amount |
| buy_orders_value | required | The combined value of all pending buy orders. | Amount |
| collateral | required | The collateral claim for options. | Amount |
| credit_account_interest | optional | The accrued interest for credit account if available. | Amount |
| credit_account_sum | optional | The sum for credit account if available. | Amount |
| equity | required | The sum of own_capital and credit_account_sum. | Amount |
| forward_sum | required | The locked amount for forwards. | Amount |
| full_marketvalue | required | The total market value. | Amount |
| future_sum | required | The sum of intraday realized profits/losses for futures in account currency. This is calculated for positions that are being closed out whether in part or entirely. The latest known foreign exchange rate is used. Reset at night. It differs from unrealized_future_profit_loss which looks at existing positions in order to calculate its value. | Amount |
| interest | required | The interest on the account. | Amount |
| intraday_credit | optional | The intraday credit if available. | Amount |
| loan_limit | required | The maximum loan limit, regardless of pawn value. | Amount |
| own_capital | required | The sum of account_sum, full_marketvalue, interest, forward_sum, future_sum and unrealized_future_profit_loss. | Amount |
| own_capital_morning | required | Own capital calculated in the morning. Does not change during the day. | Amount |
| pawn_value | required | The pawn value of all positions combined. | Amount |
| registration_date | optional | The registration date of the account formatted as YYYY-MM-DD. | string(date) |
| reserved | required | Summary of reserved trading power. | Reserved |
| short_positions_margin | optional | The short position margin if available. | Amount |
| trading_power | required | The amount available for trading. | Amount |
| unrealized_future_profit_loss | required | The sum of profit and loss for all currently existing futures positions. Not the same as future_sum which deals with contracts that are closed out during the day, i.e. realized profits/losses. | Amount |
