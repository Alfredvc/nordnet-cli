# Instrument

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| asset_class | optional | Asset class key word. | string |
| brochure_url | optional | URL to brochure if available. | string |
| currency | required | The currency of the instrument. | string |
| dividend_policy | optional | The dividend policy. | string |
| expiration_date | optional | Expiration date if applicable. | string(date) |
| instrument_group_type | optional | The instrument group. Wider description than instrument type. The description is available in the instrument type lookup. | string |
| instrument_id | required | Unique identifier of the instrument. Can be 0 if the instrument is not tradable. | integer(int64) |
| instrument_type | required | The instrument type. | string |
| isin_code | optional | The instrument ISIN code. | string |
| key_information_documents | optional | URLs to key information documents (KIDs) if available. | KeyInformationDocuments |
| leverage_percentage | optional | The leverage percentage if applicable. | number(double) |
| margin_percentage | optional | The margin percentage if applicable. | number(double) |
| market_view | optional | Marking market view for leverage instruments. U for up and D for down. | string |
| mifid2_category | optional | The MiFID II category of the instrument. Used to determine if a user can trade the instrument. | integer(int32) |
| multiplier | optional | The instrument multiplier. | number(double) |
| name | required | The instrument name. | string |
| number_of_securities | optional | Number of securities, not available for all instruments. | number(double) |
| pawn_percentage | optional | The pawn percentage if applicable. | number(double) |
| price_type | optional | Price type when trading. Not available for all markets. Examples: monetary_amount, percentage, yield. | string |
| prospectus_url | optional | URL to prospectus if available | string |
| sector | optional | The sector ID of the instrument. | string |
| sector_group | optional | The sector group of the instrument. | string |
| sfdr_article | optional | The SFDR article of a fund. Can be 6, 8 or 9. | integer(int32) |
| strike_price | optional | Strike price if applicable. | number(double) |
| symbol | required | The instrument symbol, e.g ERIC B | string |
| total_fee | optional | Total fee. | number(double) |
| tradables | optional | The tradables that belongs to the instrument. If the instrument is not tradable this field is left out. | < Tradable > array |
| underlyings | optional | A list of underlyings to the instrument. | < UnderlyingInfo > array |
