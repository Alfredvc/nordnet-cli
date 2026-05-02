# Ledger

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| acc_int_cred | required | The interest credit in the ledger currency. | Amount |
| acc_int_deb | required | The interest debit in the ledger currency. | Amount |
| account_sum | required | The sum in the ledger currency. | Amount |
| account_sum_acc | required | The sum in the account currency. | Amount |
| currency | required | The currency of the ledger. | string |
| exchange_rate | required | The price to convert to base currency. | Amount |
