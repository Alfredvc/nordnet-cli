# NewsSource

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| countries | optional | List containing the country codes affected by the news source. | < string > array |
| level | required | Valid access levels: DELAYED (news access with a 15 minute delay), REALTIME (real-time news access), FLASH (flash news access - implies real-time access for ordinary news). | string |
| name | required | News source name. | string |
| source_id | required | Nordnet unique news source ID. | integer(int64) |
