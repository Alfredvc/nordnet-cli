# MainSearchResponseRow

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| agency | optional | News agency. | string |
| agency_description | optional | Localized news agency description. | string |
| avatar_uri | optional | Shareville avatar URI. | string |
| close_price | optional | Close price value for the previous trading day. | PriceWithDecimals |
| country | optional | Shareville profile country. | string |
| currency | optional | Instrument currency. | string |
| diff_pct_one_day | optional | Yield for one day in percent. | string |
| diff_pct_one_year | optional | Yield for one year in percent. | string |
| display_name | required | Display name. | string |
| display_name_highlighted | optional | Display name with highlight tags. | string |
| display_symbol | optional | Display symbol. | string |
| entity_type | optional | Indicator entity type. For example, COMMODITY. | string |
| etp_info | optional | ETP information. | EtpInfo |
| exchange_country | optional | Exchange country code. | string |
| external_news_id | optional | External unique news ID. | integer(int64) |
| indicator_identifier | optional | Indicator ID. | string |
| indicator_source | optional | Indicator source ID. | string |
| instrument_group_type | optional | Instrument group type. | string |
| instrument_id | optional | Unique instrument ID. | integer(int64) |
| instrument_type | optional | Instrument type. | string |
| is_cms | optional | True if the page is a CMS page. | boolean |
| is_external | optional | True if the page is an external page. | boolean |
| joined_at | optional | Shareville user join date. | integer(int64) |
| ko_info | optional | Information related to knock-out instruments | KoInfo |
| language | optional | Language of the news article or page. | string |
| last_price | optional | Current last price value. | PriceWithDecimals |
| last_price_title | optional | Last price title. For example, "Senaste NAV". | string |
| market_data_order_book_id | optional | Market data order book ID used in NNX. | string |
| market_info | optional | Market information. Specifies which market the price information is collected from. | MarketInfo |
| news_id | optional | News ID as UUID used in NNX. | string |
| news_type | optional | News type. | string |
| news_type_description | optional | Localized news type description. | string |
| nnx_instrument_id | optional | Instrument ID used in NNX. | string |
| price_ko_info | optional | Knock-out instrument price information. | PriceKoInfo |
| profile_id | optional | UUID for Shareville profile. | string |
| published_date_time | optional | Publication date according to the news source. | integer(int64) |
| rating | optional | Shareville rating. | string |
| spread | optional | Bid-ask spread. | PriceWithDecimals |
| spread_pct | optional | Bid-ask spread in percent. | number(double) |
| status_info | optional | Current market trading status. | StatusInfo |
| tick_timestamp | optional | Price time stamp. | integer(int64) |
| trading_order_book_id | optional | Trading order book ID used in NNX. | string |
| turnover | optional | Daily turnover. | number(double) |
| turnover_volume | optional | Turnover volume. | integer(int64) |
| uri | optional | Page URI. | string |
| username | optional | Shareville username. | string |
| views | optional | Number of times news article has been viewed. | integer(int32) |
| yield_1y | optional | 1-day yield. | string |
