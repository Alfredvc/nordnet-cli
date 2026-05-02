# FilterDetails

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| attribute | optional | Attribute ID. | string |
| parent_filters | optional | List of attribute IDs which are logical parent filters for this attribute. (E.g. market_id is a parent of market_sub_id.) | < string > array |
| requires_expand | optional | When requires_expand=true, the attribute ID must be provided to the attribute search APIs via expand if filter values should be returned. | boolean |
| values | optional | List of filter values for this attribute, if expand is specified. Supports only the MULTISELECT filter type. | < FilterVal > array |
