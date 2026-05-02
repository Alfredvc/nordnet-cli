# InstrumentInfo

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| clearing_place | optional | Clearing place. | string |
| currency | optional | Currency. | string |
| display_name | optional | Country-specific name. Populated only for indicators. | string |
| instrument_group_type | optional | Instrument group type. | string |
| instrument_id | optional | Unique instrument ID. | integer(int64) |
| instrument_pawn_percentage | optional | Maximum pawn percentage. | integer(int32) |
| instrument_type | optional | Instrument type. | string |
| instrument_type_hierarchy | optional | Instrument type hierarchy. | string |
| is_monthly_saveable | optional | Set to true if the instrument can be used for monthly savings. | boolean |
| is_shortable | optional | Set to true if the instrument is shortable. | boolean |
| is_tradable | optional | Set to true if the instrument is tradable. For example, the instrument may be an untradable index or a tradable stock. | boolean |
| isin | optional | International securities identification number (ISIN.) | string |
| issuer_id | optional | Issuer ID. | integer(int64) |
| issuer_name | optional | Issuer name. | string |
| long_name | optional | Localized long instrument name. | string |
| name | optional | Short instrument name. | string |
| price_unit | optional | Price unit. | string |
| symbol | optional | Instrument symbol. Intended for presentation. | string |
