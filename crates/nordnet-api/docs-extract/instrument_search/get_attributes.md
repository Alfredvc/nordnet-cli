# get_attributes — Search for attributes available in the instrument search APIs.

## Endpoint

`GET /api/2/instrument_search/attributes`

## Description

Returns attributes available in the instrument search APIs.

## Parameters

| Type | Name | Description | Schema | Default |
|------|------|-------------|--------|----------|
| Header | Accept-Language optional | Language preferred in the response. Overrides the session language. Note that nb and nn are equivalent to no. | enum (da, de, en, fi, nb, nn, no, sv) |  |
| Header | Authorization optional | Basic authorization header, that is the value must be Basic <credentials> where <credentials> is the base64 encoded string <session_id>:<session_id>. | string |  |
| Query | apply_filters optional | Specifies which filters to apply to the search. | string |  |
| Query | attribute_group optional | Returns only attributes belonging to the specified attribute groups. | < enum (EXCHANGE_INFO, HISTORICAL_RETURN_INFO, INSTRUMENT_INFO, KEY_RATIOS_INFO, PRICE_INFO, MARKET_INFO, OPTION_INFO, UNDERLYING_INFO, FUND_INFO, PRICE_KO_INFO, DERIVATIVE_INFO, ETP_INFO, KO_INFO, CERTIFICATES_INFO, STATISTICAL_INFO, COMPANY_INFO, STATUS_INFO) > array |  |
| Query | entity_type optional | Returns only attributes belonging to the specified entity type. | enum (STOCKLIST, OPTIONLIST, FUTUREFORWARDLIST, BULLBEARLIST, MINIFUTURELIST, UNLIMITEDTURBOLIST, WARRANTLIST, ASSET, FUNDLIST, DANISHINVESTFUNDLIST, ETFLIST) |  |
| Query | expand optional | Expands attribute values only for the listed attributes. The default expand value is all. | < string > array |  |
| Query | minmax optional | Returns minimum and maximum values for the specified attributes. | < string > array |  |
| Query | only_filterable optional | Returns only filterable attributes. | boolean | "false" |
| Query | only_returnable optional | Returns only returnable attributes. | boolean | "false" |
| Query | only_sortable optional | Returns only sortable attributes. | boolean | "false" |

## Request Body Schema

_(none)_

## Response Body Schema

- **200**: AttributeResults

## Status Codes

| HTTP Code | Description | Schema |
|-----------|-------------|--------|
| 200 | Standard response for successful HTTP requests. | AttributeResults |
| 400 | Invalid parameter. | ErrorResponse |
| 401 | Invalid session. | ErrorResponse |
| 429 | Too Many Requests. Please wait for 10 seconds before retrying. | ErrorResponse |
| 503 | Service Unavailable. Follow the Retry-After header and retry in the specified amount of seconds. | ErrorResponse |

## Examples

_(no example blocks in documentation HTML)_

## Doc inconsistencies

_(none identified during Phase 1 extraction)_

## Referenced types

- [AttributeResults](../_definitions/AttributeResults.md)
- [ErrorResponse](../_definitions/ErrorResponse.md)
