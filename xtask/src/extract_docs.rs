//! Phase 1 doc extraction: parse the saved Nordnet API HTML and emit
//! per-operation markdown slices + INVENTORY.md.
//!
//! Design:
//! - Parse the HTML with `scraper`.
//! - Walk every `<h3>` whose `id` attribute is in the known operation ID list.
//! - Extract method + path, parameter table, response table, description.
//! - Write `docs-extract/<group>/<op>.md` for each non-deprecated operation.
//! - Write `docs-extract/INVENTORY.md` with the full 45-op table.
//! - Because the HTML contains no `<pre class="example">` JSON bodies,
//!   fixture files are not written here; Phase 2 handles fixture assembly.

use anyhow::{bail, Context, Result};
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Operation metadata table
// ---------------------------------------------------------------------------

/// All 45 documented operations, in document order.
/// Fields: (anchor_id, group, op_name, method, path_suffix, deprecated)
/// path_suffix is the portion after /api/2 (e.g. "" for root, "/accounts").
static OPERATIONS: &[OpMeta] = &[
    // root
    OpMeta {
        anchor: "_get_status",
        group: "root",
        op_name: "get_system_status",
        deprecated: false,
    },
    // accounts
    OpMeta {
        anchor: "_get_accounts",
        group: "accounts",
        op_name: "list_accounts",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_account",
        group: "accounts",
        op_name: "get_account",
        deprecated: true,
    },
    OpMeta {
        anchor: "_get_account_info",
        group: "accounts",
        op_name: "get_account_info",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_ledgers",
        group: "accounts",
        op_name: "list_ledgers",
        deprecated: false,
    },
    // orders (under accounts path)
    OpMeta {
        anchor: "_create_order",
        group: "orders",
        op_name: "place_order",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_orders",
        group: "orders",
        op_name: "list_orders",
        deprecated: false,
    },
    OpMeta {
        anchor: "_modify_order",
        group: "orders",
        op_name: "modify_order",
        deprecated: false,
    },
    OpMeta {
        anchor: "_delete_order",
        group: "orders",
        op_name: "cancel_order",
        deprecated: false,
    },
    OpMeta {
        anchor: "_order_activation",
        group: "orders",
        op_name: "activate_order",
        deprecated: false,
    },
    // accounts continued
    OpMeta {
        anchor: "_get_positions",
        group: "accounts",
        op_name: "list_positions",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_returns_transactions_today",
        group: "accounts",
        op_name: "get_returns_today",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_trades",
        group: "accounts",
        op_name: "list_trades",
        deprecated: false,
    },
    // countries
    OpMeta {
        anchor: "_get_countries",
        group: "countries",
        op_name: "list_countries",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_country",
        group: "countries",
        op_name: "get_country",
        deprecated: false,
    },
    // instrument_search
    OpMeta {
        anchor: "_get_instrument_search_attributes",
        group: "instrument_search",
        op_name: "get_attributes",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_instrument_search_bullbearlist",
        group: "instrument_search",
        op_name: "search_bullbearlist",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_instrument_search_minifuturelist",
        group: "instrument_search",
        op_name: "search_minifuturelist",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_instrument_search_optionlist_pairs",
        group: "instrument_search",
        op_name: "search_optionlist_pairs",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_instrument_search_stocklist",
        group: "instrument_search",
        op_name: "search_stocklist",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_instrument_search_unlimitedturbolist",
        group: "instrument_search",
        op_name: "search_unlimitedturbolist",
        deprecated: false,
    },
    // instruments
    OpMeta {
        anchor: "_get_instrument_lookup",
        group: "instruments",
        op_name: "lookup",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_instrument_types",
        group: "instruments",
        op_name: "list_types",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_instrument_type",
        group: "instruments",
        op_name: "get_type",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_underlying",
        group: "instruments",
        op_name: "list_underlyings",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_instruments_validation_suitability",
        group: "instruments",
        op_name: "get_suitability",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_instrument",
        group: "instruments",
        op_name: "get_instrument",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_leverages",
        group: "instruments",
        op_name: "list_leverages",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_leverage_filters",
        group: "instruments",
        op_name: "get_leverage_filters",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_instrument_public_trades",
        group: "instruments",
        op_name: "list_trades",
        deprecated: false,
    },
    // login
    OpMeta {
        anchor: "_touch_session",
        group: "login",
        op_name: "refresh_session",
        deprecated: false,
    },
    OpMeta {
        anchor: "_logout",
        group: "login",
        op_name: "logout",
        deprecated: false,
    },
    OpMeta {
        anchor: "_start_api_key_challenge",
        group: "login",
        op_name: "start_login",
        deprecated: false,
    },
    OpMeta {
        anchor: "_verify_api_key_challenge",
        group: "login",
        op_name: "verify_login",
        deprecated: false,
    },
    // main_search
    OpMeta {
        anchor: "_main_search",
        group: "main_search",
        op_name: "search",
        deprecated: false,
    },
    // markets
    OpMeta {
        anchor: "_get_markets",
        group: "markets",
        op_name: "list_markets",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_market",
        group: "markets",
        op_name: "get_market",
        deprecated: false,
    },
    // news
    OpMeta {
        anchor: "_get_news_preview",
        group: "news",
        op_name: "list_news",
        deprecated: true,
    },
    OpMeta {
        anchor: "_get_news_article",
        group: "news",
        op_name: "get_news_item",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_news_sources",
        group: "news",
        op_name: "list_news_sources",
        deprecated: false,
    },
    // tick_sizes
    OpMeta {
        anchor: "_get_ticksizes",
        group: "tick_sizes",
        op_name: "list_tick_sizes",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_ticksize",
        group: "tick_sizes",
        op_name: "get_tick_size",
        deprecated: false,
    },
    // tradables
    OpMeta {
        anchor: "_get_tradable_info",
        group: "tradables",
        op_name: "get_tradable_info",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_tradable_public_trades",
        group: "tradables",
        op_name: "list_trades",
        deprecated: false,
    },
    OpMeta {
        anchor: "_get_tradable_validation_suitability",
        group: "tradables",
        op_name: "get_suitability",
        deprecated: false,
    },
];

struct OpMeta {
    anchor: &'static str,
    group: &'static str,
    op_name: &'static str,
    deprecated: bool,
}

// ---------------------------------------------------------------------------
// HTML selectors (constructed once, reused)
// ---------------------------------------------------------------------------

struct Selectors {
    h3: Selector,
    table: Selector,
    td: Selector,
    th: Selector,
    tr: Selector,
    p: Selector,
}

impl Selectors {
    fn new() -> Self {
        Self {
            h3: Selector::parse("h3").unwrap(),
            table: Selector::parse("table").unwrap(),
            td: Selector::parse("td").unwrap(),
            th: Selector::parse("th").unwrap(),
            tr: Selector::parse("tr").unwrap(),
            p: Selector::parse("p").unwrap(),
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run(html_path: &Path) -> Result<()> {
    let workspace_root = crate::workspace_root()?;
    let docs_extract_root = workspace_root.join("crates/nordnet-api/docs-extract");
    // fixtures/ is populated in Phase 2 from example bodies. The HTML has
    // no <pre class="example"> blocks, so no fixture files are written here.

    let html_content = fs::read_to_string(html_path)
        .with_context(|| format!("reading HTML from {}", html_path.display()))?;

    let document = Html::parse_document(&html_content);
    let sel = Selectors::new();

    // Build a map: anchor_id -> (method, relative_path, section_html_start, title)
    let section_map = build_section_map(&document, &html_content, &sel)?;

    let mut inventory_rows: Vec<InventoryRow> = Vec::new();

    for op in OPERATIONS {
        let section_info = section_map.get(op.anchor).with_context(|| {
            format!(
                "anchor '{}' not found in HTML — check that HTML is up to date",
                op.anchor
            )
        })?;

        let row = InventoryRow {
            group: op.group,
            op_name: op.op_name,
            method: section_info.method.clone(),
            path: section_info.path.clone(),
            deprecated: op.deprecated,
        };
        inventory_rows.push(row);

        if op.deprecated {
            // Deprecated ops appear in INVENTORY.md but no markdown file is produced.
            continue;
        }

        // Extract the full section content from the raw HTML.
        let section_content = extract_section_content(&html_content, op.anchor);

        // Parse the section into structured parts.
        let parsed = parse_section(&section_content, &sel)?;

        // Render the per-op markdown.
        let md = render_markdown(op, section_info, &parsed);

        // Write the file.
        let group_dir = docs_extract_root.join(op.group);
        fs::create_dir_all(&group_dir)
            .with_context(|| format!("creating dir {}", group_dir.display()))?;
        let md_path = group_dir.join(format!("{}.md", op.op_name));
        crate::write_if_changed(&md_path, &md)?;
    }

    // Write INVENTORY.md.
    let inventory_md = render_inventory(&inventory_rows);
    let inventory_path = docs_extract_root.join("INVENTORY.md");
    fs::create_dir_all(&docs_extract_root)
        .with_context(|| format!("creating {}", docs_extract_root.display()))?;
    crate::write_if_changed(&inventory_path, &inventory_md)?;

    println!(
        "extract-docs complete: {} ops ({} non-deprecated), INVENTORY.md written",
        OPERATIONS.len(),
        OPERATIONS.iter().filter(|o| !o.deprecated).count()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Section map building
// ---------------------------------------------------------------------------

struct SectionInfo {
    method: String,
    path: String,
    title: String,
}

/// Walk every <h3 id="..."> element, match against our known anchor list,
/// and record the HTTP method + path by looking at the first <pre> sibling
/// that follows the h3 within the same sect2 div.
fn build_section_map(
    document: &Html,
    html_content: &str,
    sel: &Selectors,
) -> Result<HashMap<String, SectionInfo>> {
    let anchor_set: std::collections::HashSet<&str> = OPERATIONS.iter().map(|o| o.anchor).collect();

    let mut map: HashMap<String, SectionInfo> = HashMap::new();

    for h3 in document.select(&sel.h3) {
        let id = match h3.value().attr("id") {
            Some(id) if anchor_set.contains(id) => id.to_string(),
            _ => continue,
        };

        let title = h3.text().collect::<String>();

        // The <pre> containing the endpoint URL is in the next literalblock div.
        // We search in the raw HTML by anchor to avoid complex sibling traversal.
        let endpoint_url = find_endpoint_url_for_anchor(html_content, &id);
        let (method, path) = match endpoint_url {
            Some(url) => parse_endpoint_url(&url),
            None => bail!(
                "No endpoint URL found for anchor '{}'. \
                 The saved HTML may be missing this operation.",
                id
            ),
        };

        map.insert(
            id,
            SectionInfo {
                method,
                path,
                title,
            },
        );
    }

    // Verify all anchors were found.
    for op in OPERATIONS {
        if !map.contains_key(op.anchor) {
            bail!(
                "Anchor '{}' was not found in the HTML document. \
                 Update the HTML or the OPERATIONS table.",
                op.anchor
            );
        }
    }

    Ok(map)
}

/// Find the first endpoint URL `<METHOD> https://...` that immediately
/// follows the `<h3 id="<anchor>">` element.
fn find_endpoint_url_for_anchor(html: &str, anchor: &str) -> Option<String> {
    let search = format!("id=\"{}\"", anchor);
    let anchor_pos = html.find(&search)?;

    // Look for the first <pre> after the anchor.
    let pre_start = html[anchor_pos..].find("<pre>")?;
    let pre_abs = anchor_pos + pre_start + 5; // past "<pre>"
    let pre_end = html[pre_abs..].find("</pre>")?;
    let pre_content = html[pre_abs..pre_abs + pre_end].trim().to_string();

    // Verify it looks like an endpoint URL.
    if pre_content.starts_with("GET ")
        || pre_content.starts_with("POST ")
        || pre_content.starts_with("PUT ")
        || pre_content.starts_with("DELETE ")
    {
        Some(pre_content)
    } else {
        None
    }
}

/// Parse `"GET https://public.nordnet.se/api/2/accounts"` into
/// `("GET", "/accounts")`.
///
/// For the root endpoint `GET https://public.nordnet.se/api/2`, the path is
/// returned as an empty string (meaning the endpoint IS `/api/2`).
fn parse_endpoint_url(url: &str) -> (String, String) {
    let parts: Vec<&str> = url.splitn(2, ' ').collect();
    if parts.len() != 2 {
        return (url.to_string(), String::new());
    }
    let method = parts[0].to_string();
    let full_url = parts[1];
    // Strip the base URL prefix; what remains is the path suffix after /api/2.
    let base = "https://public.nordnet.se/api/2";
    let path = if let Some(stripped) = full_url.strip_prefix(base) {
        // stripped is "" for root, or "/accounts" etc for others.
        stripped.to_string()
    } else {
        full_url.to_string()
    };
    (method, path)
}

// ---------------------------------------------------------------------------
// Section content extraction
// ---------------------------------------------------------------------------

/// Extract the raw HTML for a single operation section.
/// The section starts at `id="<anchor>"` and ends just before the next `<h3`.
fn extract_section_content(html: &str, anchor: &str) -> String {
    let search = format!("id=\"{}\"", anchor);
    let start = match html.find(&search) {
        Some(pos) => pos,
        None => return String::new(),
    };
    // Find the next <h3 after our anchor.
    let remainder = &html[start..];
    // Skip past the h3 opening tag itself (find its closing >) then find the next <h3.
    let h3_close = remainder.find('>').unwrap_or(0);
    let after_h3 = &remainder[h3_close + 1..];
    let next_h3 = after_h3.find("<h3 ").unwrap_or(after_h3.len());
    remainder[..h3_close + 1 + next_h3].to_string()
}

// ---------------------------------------------------------------------------
// Parsed section
// ---------------------------------------------------------------------------

struct ParsedSection {
    description: String,
    parameters: Vec<[String; 4]>, // [type, name, description, schema]
    responses: Vec<[String; 3]>,  // [code, description, schema]
    /// True if the parameters table has a 5-column "Default" column.
    params_has_default: bool,
    /// Parameter defaults when params_has_default is true: [type, name, desc, schema, default]
    parameters_with_default: Vec<[String; 5]>,
    examples: Vec<ExampleBlock>,
}

struct ExampleBlock {
    label: String,
    content: String,
}

fn parse_section(section_html: &str, sel: &Selectors) -> Result<ParsedSection> {
    let fragment = Html::parse_fragment(section_html);

    // Extract description.
    let description = extract_description(&fragment, sel);

    // Find all tables in the section.
    let tables: Vec<_> = fragment.select(&sel.table).collect();

    // First table = parameters (if present).
    // Second table = responses (if present).
    // Heuristic: check headers.
    let mut parameters: Vec<[String; 4]> = Vec::new();
    let mut parameters_with_default: Vec<[String; 5]> = Vec::new();
    let mut params_has_default = false;
    let mut responses: Vec<[String; 3]> = Vec::new();

    for table in &tables {
        let headers = extract_table_headers(table, sel);
        if headers.is_empty() {
            continue;
        }
        let first_header = headers[0].trim().to_lowercase();
        if first_header == "type" && headers.len() >= 3 {
            // Parameters table
            if headers.len() >= 5 && headers[4].trim().to_lowercase() == "default" {
                params_has_default = true;
                parameters_with_default = extract_table_rows_5(table, sel);
            } else {
                parameters = extract_table_rows_4(table, sel);
            }
        } else if first_header == "http code" {
            // Responses table
            responses = extract_table_rows_3(table, sel);
        }
    }

    // There are no example blocks in this HTML (no <pre class="example">).
    let examples: Vec<ExampleBlock> = Vec::new();

    Ok(ParsedSection {
        description,
        parameters,
        params_has_default,
        parameters_with_default,
        responses,
        examples,
    })
}

fn extract_description(fragment: &Html, sel: &Selectors) -> String {
    // Grab the first non-table paragraph in the section — that is the description.
    // Table cell paragraphs have class "tableblock"; we skip those.
    for p in fragment.select(&sel.p) {
        let class = p.value().attr("class").unwrap_or("");
        if class.contains("tableblock") {
            continue;
        }
        let trimmed = p.text().collect::<String>().trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    String::new()
}

fn extract_table_headers(table: &scraper::ElementRef, sel: &Selectors) -> Vec<String> {
    let mut headers = Vec::new();
    for tr in table.select(&sel.tr) {
        for th in tr.select(&sel.th) {
            headers.push(th.text().collect::<String>().trim().to_string());
        }
        if !headers.is_empty() {
            break;
        }
    }
    headers
}

fn extract_table_rows_4(table: &scraper::ElementRef, sel: &Selectors) -> Vec<[String; 4]> {
    let mut rows = Vec::new();
    let mut first = true;
    for tr in table.select(&sel.tr) {
        // Skip header row
        let has_th = tr.select(&sel.th).next().is_some();
        if has_th {
            first = false;
            continue;
        }
        if first {
            first = false;
            continue;
        }
        let cells: Vec<String> = tr
            .select(&sel.td)
            .map(|td| clean_cell_text(td.text().collect::<String>()))
            .collect();
        if cells.len() >= 4 {
            rows.push([
                cells[0].clone(),
                cells[1].clone(),
                cells[2].clone(),
                cells[3].clone(),
            ]);
        } else {
            // Pad shorter rows
            let mut padded = cells.clone();
            while padded.len() < 4 {
                padded.push(String::new());
            }
            rows.push([
                padded[0].clone(),
                padded[1].clone(),
                padded[2].clone(),
                padded[3].clone(),
            ]);
        }
    }
    rows
}

fn extract_table_rows_5(table: &scraper::ElementRef, sel: &Selectors) -> Vec<[String; 5]> {
    let mut rows = Vec::new();
    for tr in table.select(&sel.tr) {
        let has_th = tr.select(&sel.th).next().is_some();
        if has_th {
            continue;
        }
        let cells: Vec<String> = tr
            .select(&sel.td)
            .map(|td| clean_cell_text(td.text().collect::<String>()))
            .collect();
        if cells.len() >= 5 {
            rows.push([
                cells[0].clone(),
                cells[1].clone(),
                cells[2].clone(),
                cells[3].clone(),
                cells[4].clone(),
            ]);
        } else if cells.len() >= 4 {
            rows.push([
                cells[0].clone(),
                cells[1].clone(),
                cells[2].clone(),
                cells[3].clone(),
                String::new(),
            ]);
        }
    }
    rows
}

fn extract_table_rows_3(table: &scraper::ElementRef, sel: &Selectors) -> Vec<[String; 3]> {
    let mut rows = Vec::new();
    for tr in table.select(&sel.tr) {
        let has_th = tr.select(&sel.th).next().is_some();
        if has_th {
            continue;
        }
        let cells: Vec<String> = tr
            .select(&sel.td)
            .map(|td| clean_cell_text(td.text().collect::<String>()))
            .collect();
        if cells.len() >= 3 {
            rows.push([cells[0].clone(), cells[1].clone(), cells[2].clone()]);
        } else if cells.len() == 2 {
            rows.push([cells[0].clone(), cells[1].clone(), String::new()]);
        }
    }
    rows
}

fn clean_cell_text(raw: String) -> String {
    // Collapse multiple whitespace sequences into a single space.
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ---------------------------------------------------------------------------
// Markdown rendering
// ---------------------------------------------------------------------------

fn render_markdown(op: &OpMeta, info: &SectionInfo, parsed: &ParsedSection) -> String {
    let mut md = String::new();

    // Title
    md.push_str(&format!("# {} — {}\n\n", op.op_name, info.title));

    // Endpoint
    md.push_str("## Endpoint\n\n");
    md.push_str(&format!("`{} /api/2{}`\n\n", info.method, info.path));
    // Note: for the root endpoint, info.path is "" so the above yields `GET /api/2`.

    // Description
    if !parsed.description.is_empty() {
        md.push_str("## Description\n\n");
        md.push_str(&parsed.description);
        md.push_str("\n\n");
    }

    // Parameters table
    if parsed.params_has_default && !parsed.parameters_with_default.is_empty() {
        md.push_str("## Parameters\n\n");
        md.push_str("| Type | Name | Description | Schema | Default |\n");
        md.push_str("|------|------|-------------|--------|----------|\n");
        for row in &parsed.parameters_with_default {
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                escape_md(&row[0]),
                escape_md(&row[1]),
                escape_md(&row[2]),
                escape_md(&row[3]),
                escape_md(&row[4])
            ));
        }
        md.push('\n');
    } else if !parsed.parameters.is_empty() {
        md.push_str("## Parameters\n\n");
        md.push_str("| Type | Name | Description | Schema |\n");
        md.push_str("|------|------|-------------|--------|\n");
        for row in &parsed.parameters {
            md.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                escape_md(&row[0]),
                escape_md(&row[1]),
                escape_md(&row[2]),
                escape_md(&row[3])
            ));
        }
        md.push('\n');
    } else {
        md.push_str("## Parameters\n\n");
        md.push_str("_(none)_\n\n");
    }

    // Request body schema
    // For POST/PUT operations with FormData parameters, extract them.
    let form_params: Vec<_> = if parsed.params_has_default {
        parsed
            .parameters_with_default
            .iter()
            .filter(|r| {
                r[0].to_lowercase().contains("formdata") || r[0].to_lowercase().contains("body")
            })
            .map(|r| format!("- **{}** ({}) — {}", r[1], r[3], r[2]))
            .collect()
    } else {
        parsed
            .parameters
            .iter()
            .filter(|r| {
                r[0].to_lowercase().contains("formdata") || r[0].to_lowercase().contains("body")
            })
            .map(|r| format!("- **{}** ({}) — {}", r[1], r[3], r[2]))
            .collect()
    };

    if !form_params.is_empty() {
        md.push_str("## Request Body Schema\n\n");
        md.push_str("_(form data parameters)_\n\n");
        for p in &form_params {
            md.push_str(p);
            md.push('\n');
        }
        md.push('\n');
    } else {
        md.push_str("## Request Body Schema\n\n");
        md.push_str("_(none)_\n\n");
    }

    // Response body schema — extract from responses table.
    let success_schema: Vec<_> = parsed
        .responses
        .iter()
        .filter(|r| r[0].starts_with('2'))
        .map(|r| format!("- **{}**: {}", r[0], r[2]))
        .collect();
    md.push_str("## Response Body Schema\n\n");
    if success_schema.is_empty() {
        md.push_str("_(see Status Codes table)_\n\n");
    } else {
        for s in &success_schema {
            md.push_str(s);
            md.push('\n');
        }
        md.push('\n');
    }

    // Status codes
    if !parsed.responses.is_empty() {
        md.push_str("## Status Codes\n\n");
        md.push_str("| HTTP Code | Description | Schema |\n");
        md.push_str("|-----------|-------------|--------|\n");
        for row in &parsed.responses {
            md.push_str(&format!(
                "| {} | {} | {} |\n",
                escape_md(&row[0]),
                escape_md(&row[1]),
                escape_md(&row[2])
            ));
        }
        md.push('\n');
    }

    // Example blocks (none in this HTML, but section present for Phase 2).
    md.push_str("## Examples\n\n");
    if parsed.examples.is_empty() {
        md.push_str("_(no example blocks in documentation HTML)_\n\n");
    } else {
        for ex in &parsed.examples {
            md.push_str(&format!("### {}\n\n", ex.label));
            md.push_str("```\n");
            md.push_str(&ex.content);
            md.push_str("\n```\n\n");
        }
    }

    // Doc inconsistencies (empty — filled in during Phase 2C).
    md.push_str("## Doc inconsistencies\n\n");
    md.push_str("_(none identified during Phase 1 extraction)_\n");

    md
}

fn escape_md(s: &str) -> String {
    // Escape pipe characters in table cells.
    s.replace('|', "\\|")
}

// ---------------------------------------------------------------------------
// INVENTORY.md
// ---------------------------------------------------------------------------

struct InventoryRow {
    group: &'static str,
    op_name: &'static str,
    method: String,
    path: String,
    deprecated: bool,
}

fn render_inventory(rows: &[InventoryRow]) -> String {
    let mut md = String::new();
    md.push_str("# Nordnet API v2 — Operation Inventory\n\n");
    md.push_str("Generated by `cargo xtask extract-docs`. Do not hand-edit.\n\n");
    md.push_str(
        "Total: 45 documented operations — 2 deprecated (marked SKIP), 43 to implement.\n\n",
    );
    md.push_str("| Group | Operation | Method + Path | Deprecated |\n");
    md.push_str("|-------|-----------|---------------|------------|\n");
    for row in rows {
        let dep = if row.deprecated { "SKIP" } else { "" };
        // row.path is "" for the root endpoint, so method_path becomes "GET /api/2".
        let method_path = format!("{} /api/2{}", row.method, row.path);
        md.push_str(&format!(
            "| {} | {} | `{}` | {} |\n",
            row.group, row.op_name, method_path, dep
        ));
    }
    md.push('\n');
    md
}
