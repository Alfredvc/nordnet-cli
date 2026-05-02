# OptionlistEntity

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| derivative_info | optional | Derivative information. | DerivativeInfo |
| exchange_info | optional | Information regarding the instrument/tradable on the exchange. | ExchangeInfo |
| instrument_info | optional | Instrument information. The same content may appear in multiple search results if there are multiple tradables or fund universes. | InstrumentInfo |
| market_info | optional | Identifies which market the price information is collected from. | MarketInfo |
| option_info | optional | Option information. | OptionInfo |
| price_info | optional | Price information, representing an exchange-traded model containing top-of-book information. | PriceInfo |
