# CompanyInfo

## Fields

| Name | Required | Description | Schema |
|------|----------|-------------|--------|
| dividend_amount | optional | Upcoming dividend amount. | number(double) |
| dividend_bonus_frequency | optional | Upcoming bonus dividend frequency. | integer(int64) |
| dividend_currency | optional | Upcoming dividend currency. | string |
| dividend_date | optional | Upcoming dividend payout date. | integer(int64) |
| dividend_frequency | optional | Upcoming dividend frequency (excl. bonus dividends). | integer(int64) |
| excluding_date | optional | Upcoming dividend exclude date. | integer(int64) |
| general_meeting_date | optional | Upcoming annual general meeting date. | integer(int64) |
| report_date | optional | Upcoming report date. | integer(int64) |
| report_description | optional | Upcoming report type translation. | string |
| report_type | optional | Upcoming report type. For example, ANNUAL_REPORT, FIRST_QUARTER_EARNINGS_RESULTS. | string |
