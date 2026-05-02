# NewsArticle

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| body | optional | Article body. | string |
| byline | optional | Article author. | string |
| headline | required | Article headline. | string |
| instruments | optional | List of instrument IDs affected by article. | < integer > array |
| isin_codes | optional | List of ISINs affected by the article. | < string > array |
| lang | required | News language. | string |
| markdown_format | required | Whether the article is in markdown format. | boolean |
| markets | optional | List of market IDs affected by the article. | < integer > array |
| news_id | required | External unique news ID. | integer(int64) |
| news_type | required | Valid types are: NEWS, ANALYSIS, PRESS_RELEASE, MARKET_COMMENTARY, PM, PMVECKAN, MARKET_NEWS, VOLATILITY_HALT, TRADING_HALT, TRADING_EVENT, TOP10. | string |
| sectors | optional | List of sectors affected by the article. | < string > array |
| source_id | required | Nordnet unique news source ID. | integer(int64) |
| summary | optional | Article summary. | string |
| timestamp | required | Publication date expressed in milliseconds since January 1st 1970 00:00:00 UTC. | integer(int64) |
| type | required | Exists for backwards compatibility. Always set to NEWS. | string |
| version | required | Article version. | integer(int64) |
