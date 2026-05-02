# MinifutureEntity

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| etp_info | optional | ETP information. | EtpInfo |
| exchange_info | optional | Information regarding the instrument/tradable on the exchange. | ExchangeInfo |
| instrument_info | optional | Instrument information. The same content may appear in multiple search results if there are multiple tradables or fund universes. | InstrumentInfo |
| ko_calc_info | optional | Knock-out instrument related information. | KoCalcInfo |
| ko_info | optional | Information related to knock-out instruments. | KoInfo |
| market_info | optional | Identifies which market the price information is collected from. | MarketInfo |
| price_info | optional | Price information, representing an exchange-traded model containing top-of-book information. | PriceInfo |
| price_ko_info | optional | Knock-out instrument related price information. | PriceKoInfo |
