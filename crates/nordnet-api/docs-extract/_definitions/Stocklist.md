# Stocklist

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| company_info | optional | Company information providing upcoming activity dates and other information associated with the company. | CompanyInfo |
| exchange_info | optional | Instrument exchange information. | ExchangeInfo |
| historical_returns_info | optional | Historical returns information showing the yield over time. | HistoricalReturnsInfo |
| instrument_info | optional | Instrument information. The same instrument information may appear in multiple search results for multiple instruments or fund universes. | InstrumentInfo |
| key_ratios_info | optional | Key ratios information describing the financial condition of the company (or the share.) | KeyRatiosInfo |
| market_info | optional | Market information. Identifies the origin market for the price information. | MarketInfo |
| price_info | optional | Price information representing an exchange-traded model containing top-of-book information. | PriceInfo |
| statistical_info | optional | Statistical information on the number of Nordnet customers with positions in the instrument and the number of orders. | StatisticalInfo |
