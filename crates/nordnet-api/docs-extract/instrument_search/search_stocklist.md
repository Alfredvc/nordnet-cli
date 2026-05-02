# search_stocklist — Search for stocks

## Endpoint

`GET /api/2/instrument_search/query/stocklist`

## Description

Returns the stocks matching the given search criteria.

## Parameters

| Type | Name | Description | Schema | Default |
|------|------|-------------|--------|----------|
| Header | Accept-Language optional | Language preferred in the response. Overrides the session language. Note that nb and nn are equivalent to no. | enum (da, de, en, fi, nb, nn, no, sv) |  |
| Header | Authorization optional | Basic authorization header, that is the value must be Basic <credentials> where <credentials> is the base64 encoded string <session_id>:<session_id>. | string |  |
| Query | apply_filters optional | Defines which filters to apply to the search. | string |  |
| Query | attribute_groups optional | Returns only attributes for the given attribute groups. | < enum (DOCUMENT_INFO, EXCHANGE_INFO, HISTORICAL_RETURN_INFO, INSTRUMENT_INFO, KEY_RATIOS_INFO, PRICE_INFO, MARKET_INFO, COMPANY_INFO, STATISTICAL_INFO) > array |  |
| Query | attributes optional | Returns only the given attributes. | < string > array |  |
| Query | free_text_search optional | Free-text search string. May contain instrument name, symbol, ISIN code. | string |  |
| Query | limit optional | Limits the search results to limit. A maximum of limit * 2 results are returned. The default limit is 50. Combine with offset to implement pagination. | integer(int32) | "50" |
| Query | offset optional | Skips the first offset search results per group. The default offset is 0. Combine with limit to implement pagination. | integer(int32) | "0" |
| Query | sort_attribute optional | Defines the attribute to sort the search results by. The default sort_attribute is name. | string | "name" |
| Query | sort_order optional | Defines the sort order of the search results. Use asc for ascending order and desc for descending order. The default sort_order is asc. | enum (asc, desc) | "asc" |

## Request Body Schema

_(none)_

## Response Body Schema

- **200**: StocklistResults

## Status Codes

| HTTP Code | Description | Schema |
|-----------|-------------|--------|
| 200 | Standard response for successful HTTP requests. | StocklistResults |
| 400 | Invalid parameter. | ErrorResponse |
| 401 | Invalid session. | ErrorResponse |
| 429 | Too Many Requests. Please wait for 10 seconds before retrying. | ErrorResponse |
| 503 | Service Unavailable. Follow the Retry-After header and retry in the specified amount of seconds. | ErrorResponse |

## Examples

_(no example blocks in documentation HTML)_

## Doc inconsistencies

_(none identified during Phase 1 extraction)_
