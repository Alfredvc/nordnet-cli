# Validity

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| type | required | DAY, UNTIL_DATE, EXTENDED_HOURS or IMMEDIATE | string |
| valid_until | optional | The cancel date, only used when type is UNTIL_DATE. UNIX timestamp in milliseconds. | integer(int64) |
