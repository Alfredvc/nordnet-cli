# MainSearchResponse

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| display_group_description | required | Result group data description. | string |
| display_group_type | required | Result group data type. | string |
| limit | optional | Limit for the search results. | integer(int32) |
| offset | optional | Offset for the search results. | integer(int32) |
| results | required | Results. | < MainSearchResponseRow > array |
| total | optional | Total number of available rows. | integer(int32) |
