# AttributeResult

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| filter_details | optional | Additional details for attributes having the filterable=true flag. | FilterDetails |
| filterable | optional | Signals whether the attribute can be used as a filter in the instrument search APIs. | boolean |
| id | optional | Attribute ID. | string |
| max | optional | Maximum value. | number(double) |
| min | optional | Minumum value. | number(double) |
| name | optional | Attribute name. | string |
| returnable | optional | Signals whether the attribute can be returned by the instrument search APIs. | boolean |
| sortable | optional | Signals whether the attribute can be used for sorting by the instrument search APIs. | boolean |
