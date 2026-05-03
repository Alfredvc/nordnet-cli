//! `nordnet instrument-search` — attribute + entity-list searches.
//!
//! # Implemented ops
//!
//! - `attributes`           → `client.get_attributes(AttributesQuery)`
//! - `stocklist`            → `client.search_stocklist(StocklistQuery)`
//! - `bull-bear-list`       → `client.search_bullbearlist(ListSearchQuery)`
//! - `mini-future-list`     → `client.search_minifuturelist(ListSearchQuery)`
//! - `unlimited-turbo-list` → `client.search_unlimitedturbolist(ListSearchQuery)`
//! - `option-list-pairs`    → `client.search_optionlist_pairs(currency, expire_date, underlying_symbol)`

use clap::{ArgAction, Args, Subcommand};
use indoc::indoc;
use nordnet_api::resources::instrument_search::{AttributesQuery, ListSearchQuery, StocklistQuery};

/// Subcommands for the `instrument-search` namespace.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Search for attributes available in the instrument-search APIs.
    ///
    /// Discovery endpoint — lists every attribute the list-search
    /// endpoints can filter, sort, or return. Filter by entity type
    /// (e.g. `STOCKLIST`) to scope to one list.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet instrument-search attributes --entity-type STOCKLIST
            nordnet instrument-search attributes --only-filterable=true
            nordnet instrument-search attributes --attribute-group PRICE_INFO --attribute-group EXCHANGE_INFO
    "})]
    Attributes(AttributesArgs),
    /// Search the stock list.
    ///
    /// Free-text + structured filter search across the equity universe.
    /// Use `--attributes` to control which attributes are returned (the
    /// default response can be large).
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet instrument-search stocklist --free-text-search ericsson
            nordnet instrument-search stocklist --limit 50 --offset 0 --sort-attribute name
            nordnet instrument-search stocklist --attributes name --attributes isin
    "})]
    Stocklist(StocklistArgs),
    /// Search the bull/bear instrument list.
    ///
    /// Same shape as `stocklist` but scoped to bull/bear certificates.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet instrument-search bull-bear-list --limit 25
            nordnet instrument-search bull-bear-list --free-text-search OMXS30
    "})]
    BullBearList(ListSearchArgs),
    /// Search the mini-future instrument list.
    ///
    /// Same shape as `stocklist` but scoped to mini-futures.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet instrument-search mini-future-list --limit 25
    "})]
    MiniFutureList(ListSearchArgs),
    /// Search the unlimited-turbo instrument list.
    ///
    /// Same shape as `stocklist` but scoped to unlimited-turbos.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet instrument-search unlimited-turbo-list --limit 25
    "})]
    UnlimitedTurboList(ListSearchArgs),
    /// Look up an option-pair by underlying + expiration date + currency.
    ///
    /// `expire_date` is a UNIX-millis epoch timestamp (i64) — divide
    /// rendered dates by 1000 for the Unix-seconds equivalent.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet instrument-search option-list-pairs \\
                --currency SEK --expire-date 1735689600000 --underlying-symbol 'ERIC B'
    "})]
    OptionListPairs(OptionListPairsArgs),
}

/// Arguments for the `attributes` subcommand.
#[derive(Debug, Args)]
pub struct AttributesArgs {
    /// Specifies which filters to apply to the search.
    #[arg(long)]
    pub apply_filters: Option<String>,
    /// Returns only attributes belonging to the specified attribute group
    /// (e.g. `EXCHANGE_INFO`, `PRICE_INFO`). Pass multiple times for multiple groups.
    #[arg(long, action = ArgAction::Append)]
    pub attribute_group: Vec<String>,
    /// Returns only attributes belonging to the specified entity type
    /// (e.g. `STOCKLIST`, `OPTIONLIST`).
    #[arg(long)]
    pub entity_type: Option<String>,
    /// Expand attribute values only for the listed attributes.
    /// Pass multiple times for multiple attributes.
    #[arg(long, action = ArgAction::Append)]
    pub expand: Vec<String>,
    /// Returns minimum and maximum values for the specified attributes.
    /// Pass multiple times for multiple attributes.
    #[arg(long, action = ArgAction::Append)]
    pub minmax: Vec<String>,
    /// Returns only filterable attributes when set to `true`.
    #[arg(long)]
    pub only_filterable: Option<bool>,
    /// Returns only returnable attributes when set to `true`.
    #[arg(long)]
    pub only_returnable: Option<bool>,
    /// Returns only sortable attributes when set to `true`.
    #[arg(long)]
    pub only_sortable: Option<bool>,
}

/// Arguments for the `stocklist` subcommand.
#[derive(Debug, Args)]
pub struct StocklistArgs {
    /// Defines which filters to apply to the search.
    #[arg(long)]
    pub apply_filters: Option<String>,
    /// Returns only attributes for the given attribute group.
    /// Pass multiple times for multiple groups.
    #[arg(long, action = ArgAction::Append)]
    pub attribute_groups: Vec<String>,
    /// Returns only the given attributes. Pass multiple times for multiple attributes.
    #[arg(long, action = ArgAction::Append)]
    pub attributes: Vec<String>,
    /// Free-text search string (instrument name, symbol, or ISIN).
    #[arg(long)]
    pub free_text_search: Option<String>,
    /// Limits the search results to this many entries (server default 50).
    #[arg(long)]
    pub limit: Option<i32>,
    /// Skips the first N search results (server default 0).
    #[arg(long)]
    pub offset: Option<i32>,
    /// Defines the attribute to sort by (server default `name`).
    #[arg(long)]
    pub sort_attribute: Option<String>,
    /// Defines the sort order: `asc` or `desc` (server default `asc`).
    #[arg(long)]
    pub sort_order: Option<String>,
}

/// Arguments shared by the bull-bear, mini-future, and unlimited-turbo list subcommands.
#[derive(Debug, Args)]
pub struct ListSearchArgs {
    /// Specifies which filters to apply to the search.
    #[arg(long)]
    pub apply_filters: Option<String>,
    /// Free text search for name, symbol, and ISIN.
    #[arg(long)]
    pub free_text_search: Option<String>,
    /// Limits the search results to this many entries.
    #[arg(long)]
    pub limit: Option<i32>,
    /// Skips the first N search results.
    #[arg(long)]
    pub offset: Option<i32>,
    /// Defines the attribute to sort by.
    #[arg(long)]
    pub sort_attribute: Option<String>,
    /// Defines the sort order: `asc` or `desc`.
    #[arg(long)]
    pub sort_order: Option<String>,
}

/// Arguments for the `option-list-pairs` subcommand.
#[derive(Debug, Args)]
pub struct OptionListPairsArgs {
    /// Option currency (e.g. `SEK`).
    #[arg(long)]
    pub currency: String,
    /// Expiration date as a UNIX-millis epoch timestamp (i64).
    #[arg(long)]
    pub expire_date: i64,
    /// Underlying instrument symbol (e.g. `ERIC B`).
    #[arg(long)]
    pub underlying_symbol: String,
}

impl Cmd {
    pub async fn run(self, client: &nordnet_api::Client, fields: &[String]) -> anyhow::Result<()> {
        match self {
            Cmd::Attributes(a) => {
                let q = AttributesQuery {
                    apply_filters: a.apply_filters.as_deref(),
                    attribute_group: a.attribute_group,
                    entity_type: a.entity_type.as_deref(),
                    expand: a.expand,
                    minmax: a.minmax,
                    only_filterable: a.only_filterable,
                    only_returnable: a.only_returnable,
                    only_sortable: a.only_sortable,
                };
                let r = client.get_attributes(q).await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Stocklist(a) => {
                let q = StocklistQuery {
                    apply_filters: a.apply_filters.as_deref(),
                    attribute_groups: a.attribute_groups,
                    attributes: a.attributes,
                    free_text_search: a.free_text_search.as_deref(),
                    limit: a.limit,
                    offset: a.offset,
                    sort_attribute: a.sort_attribute.as_deref(),
                    sort_order: a.sort_order.as_deref(),
                };
                let r = client.search_stocklist(q).await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::BullBearList(a) => {
                let q = make_list_search_query(&a);
                let r = client.search_bullbearlist(q).await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::MiniFutureList(a) => {
                let q = make_list_search_query(&a);
                let r = client.search_minifuturelist(q).await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::UnlimitedTurboList(a) => {
                let q = make_list_search_query(&a);
                let r = client.search_unlimitedturbolist(q).await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::OptionListPairs(a) => {
                let r = client
                    .search_optionlist_pairs(&a.currency, a.expire_date, &a.underlying_symbol)
                    .await?;
                crate::output::emit(&r, fields)?;
            }
        }
        Ok(())
    }
}

fn make_list_search_query(a: &ListSearchArgs) -> ListSearchQuery<'_> {
    ListSearchQuery {
        apply_filters: a.apply_filters.as_deref(),
        free_text_search: a.free_text_search.as_deref(),
        limit: a.limit,
        offset: a.offset,
        sort_attribute: a.sort_attribute.as_deref(),
        sort_order: a.sort_order.as_deref(),
    }
}
