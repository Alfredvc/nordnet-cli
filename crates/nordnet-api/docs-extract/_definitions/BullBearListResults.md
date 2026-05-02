# BullBearListResults

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| results | optional | Bull & Bear search results. | < BullBearEntity > array |
| rows | optional | Number of results returned. | integer(int32) |
| total_hits | optional | Number of search hits. | integer(int64) |
| underlying_instrument_id | optional | ID of the underlying instrument if and only if the results contain instruments with the same underlying instrument. | integer(int64) |
