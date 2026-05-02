# OrderReply

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| action_state | optional | The action state. This can be missing if the order fails the prevalidation and never enters the order system. | string |
| message | optional | Translated error message if result_code is not OK. | string |
| order_id | required | The Nordnet order identifier. | integer(int64) |
| order_state | optional | The order state. Only returned for valid orders. | string |
| result_code | required | OK or error code. | string |
