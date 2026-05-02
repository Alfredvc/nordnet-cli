# Nordnet CLI — Build Process

Custom pipeline. Supersedes cli-creator default 17-step pipeline because:

- API is large (~42 non-deprecated ops, ~37 paths)
- No OpenAPI spec — types must be hand-derived from docs
- Real trading + banking API — write ops are destructive
- Two-crate workspace (`nordnet-api` lib + `nordnet-cli` bin)
- **No Nordnet API calls.** No agent in this pipeline ever calls the Nordnet API (live trading/banking endpoints). General network access (rustup, crates.io, apt, package downloads, doc fetches for non-Nordnet libraries) is allowed and expected. All Nordnet inputs come from the saved documentation HTML. All Nordnet tests run against in-process wiremock. The user is responsible for any real-Nordnet-API verification after a release is produced.

## Priority order (binding)

1. **Documentation faithfulness.** Every typed call matches what the docs state — parameter table, request body schema, response schema, status codes, example bodies. Doc inconsistencies trigger reviewer escalation, not guesses.
2. **Full non-deprecated API parity.** All ~42 ops implemented. Read + write. No staged sub-release.
3. **Token efficiency via subagent fan-out.** Each implementer touches one resource group only, reads only its slice of docs.
4. **Strict typing.** No `serde_json::Value` in public API. Newtypes for IDs. `#[serde(deny_unknown_fields)]` everywhere.
5. **Static gates.** `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --workspace` must pass at every phase boundary. Pre-commit hook enforces.

## What "correctness" means here

We cannot verify "first-try works on real Nordnet API" because no agent in this pipeline is allowed to call Nordnet. (General network for tooling is fine.) We instead deliver the strongest Nordnet-offline-achievable guarantee:

- **Type definition matches doc** — every struct field comes from the parameter/schema table; every enum variant from the documented value set.
- **Cross-source consistency within docs** — the parameter table, the response schema table, and the example body for one endpoint must agree. Disagreement = doc bug, flagged in notes, conservative pick chosen + recorded.
- **Cross-endpoint consistency** — if `Account` appears in 5 endpoints, it has one Rust type used 5 times, derived from the union of all 5 occurrences. Mismatches in field names or types between occurrences = flagged.
- **Round-trip stability** — every doc-example body parses into the type and re-serializes byte-equivalent under canonical ordering.
- **Wiremock end-to-end** — for every op, the CLI subcommand can call the lib function against a mock serving the doc-example response, and emit the documented JSON shape on stdout.

What we do **not** guarantee, by user instruction:

- That the live API matches the documentation. (Documentation may be stale/wrong; only the user can verify.)
- That auth handshake works against the live login flow.
- That a write op accepted by the documented schema is accepted by the live server.

## Workspace layout (locked)

```
nordnet-cli/                 (workspace root)
├── Cargo.toml               (workspace, foundation owns)
├── crates/
│   ├── nordnet-api/         (lib crate — REST bindings)
│   │   ├── Cargo.toml
│   │   ├── fixtures/<group>/*.json     (doc-extracted example bodies)
│   │   ├── docs-extract/<group>/*.md   (per-endpoint AsciiDoc slice)
│   │   ├── src/
│   │   │   ├── lib.rs               (foundation, locked)
│   │   │   ├── client.rs            (foundation, locked)
│   │   │   ├── auth.rs              (foundation, locked)
│   │   │   ├── error.rs             (foundation, locked)
│   │   │   ├── pagination.rs        (foundation, locked)
│   │   │   ├── ids.rs               (foundation, locked)
│   │   │   ├── models/
│   │   │   │   ├── mod.rs           (GENERATED — never hand-edit)
│   │   │   │   ├── shared.rs        (foundation, locked after Phase 0)
│   │   │   │   └── <group>.rs       (one file per resource group)
│   │   │   └── resources/
│   │   │       ├── mod.rs           (GENERATED)
│   │   │       └── <group>.rs       (one file per resource group)
│   │   └── tests/
│   │       └── <group>_test.rs      (one file per resource group)
│   └── nordnet-cli/         (bin crate — agent CLI)
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs              (foundation, locked)
│           ├── output.rs            (foundation, locked)
│           ├── config.rs            (foundation, locked)
│           └── cmd/
│               ├── mod.rs           (GENERATED)
│               ├── orders.rs        (foundation, locked — dispatcher for orders_read + orders_write)
│               └── <cli_group>.rs   (one file per CLI group; orders is split into orders_read + orders_write)
├── xtask/
│   └── src/main.rs                  (gen-mods, extract-docs, consistency-check)
├── CONTRACTS.md                     (foundation locks here, never edited after)
├── PROCESS.md                       (this file)
└── .claude/                         (cli-creator state)
```

**`mod.rs` files are auto-regenerated by `cargo xtask gen-mods`.** Subagents never edit them. Primary conflict-elimination mechanism.

## Phases

| # | Phase | Parallel? | Owns |
|---|---|---|---|
| 0 | Foundation | sequential, 1 agent | workspace, client, auth, error, ids, shared models, CONTRACTS.md, xtask |
| 1 | Doc extraction | sequential, 1 agent | `docs-extract/<group>/` — per-op markdown slices + INVENTORY.md |
| 2 | Fixture assembly | sequential, 1 agent | `fixtures/<group>/*.json` — doc-example request and response bodies |
| 2C | Cross-source consistency check | sequential, 1 agent (opus) | flags doc inconsistencies before implementers see them |
| 3 | Resource implementation | **parallel waves** | `models/<group>.rs`, `resources/<group>.rs`, `tests/<group>_test.rs` |
| 3R | Per-task code review | parallel, 1 reviewer per implementer | review notes, fix loop |
| 3X | Cross-endpoint type consistency | sequential, 1 agent (opus) | reconciles types shared across groups |
| 4 | CLI surface | parallel waves | `cmd/<group>.rs` |
| 4R | Per-task review | parallel | review notes |
| 5 | Workspace integration | sequential, 1 agent | regenerate mods, full test, lint, sequential commits |

Each phase boundary is a hard gate. Next phase only starts when previous phase's gate passes (compile + lint + test green).

---

## Phase 0 — Foundation (sequential, single agent, opus)

Builds everything every later phase depends on. Locks down APIs other agents will use.

**Outputs:**

1. `Cargo.toml` workspace + both crates with pinned deps (`reqwest`, `serde`, `serde_json`, `tokio`, `clap`, `thiserror`, `rsa`, `base64`, `wiremock`, `pretty_assertions`, `rust_decimal`, `time`).
2. `crates/nordnet-api/src/error.rs` — `Error` enum mapping every documented status (400, 401, 403, 429, 503) + transport errors. Carries response body string.
3. `crates/nordnet-api/src/ids.rs` — newtypes: `AccountId`, `InstrumentId`, `OrderId`, `MarketId`, `TickSizeId`, `TradableId`. Each `serde(transparent)` over the documented underlying type.
4. `crates/nordnet-api/src/auth.rs` — full SSH-key login flow per docs (`POST /login/start`, RSA encrypt of `username:password:timestamp`, `POST /login/verify`), session struct, `Authorization: Basic <session_id:session_id>` builder. Unit-tested for the crypto layer (deterministic input → known ciphertext bytes against a fixed RSA test key). HTTP layer wiremock-tested.
5. `crates/nordnet-api/src/client.rs` — `Client` struct holding session, base URL, `reqwest::Client`. Generic typed `get<T>`, `post<T,B>`, `put<T,B>`, `delete<T>`. 429 retry with documented 10s wait. 503 honors `Retry-After`. Single response-parse path.
6. `crates/nordnet-api/src/pagination.rs` — `Page<T>` struct (offset/limit), iterator helper.
7. `crates/nordnet-api/src/models/shared.rs` — `ErrorResponse`, `Currency`, `Money`, `Amount`, common timestamp type (`OffsetDateTime` via `time` crate, ISO 8601). Locked after Phase 0.
8. `crates/nordnet-cli/src/output.rs` — JSON-to-stdout output module + `--fields` filter.
9. `crates/nordnet-cli/src/config.rs` — env + `~/.config/nordnet/credentials.toml` loader. Holds username, password, key path, default account.
9b. `crates/nordnet-cli/src/cmd/orders.rs` — 15-line dispatcher composing `orders_read::Cmd` + `orders_write::Cmd` under the `nordnet orders ...` namespace. Locked. (See Phase 4 for shape.)
10. `xtask/src/main.rs` — three subcommands:
    - `gen-mods` regenerates all `mod.rs` from filesystem.
    - `extract-docs --html docs-source/nordnet-api-v2.html` regenerates `docs-extract/<group>/*.md` and `fixtures/<group>/*.json` from the saved HTML.
    - `consistency-check` runs cross-source + cross-endpoint checks (Phase 2C and 3X driver).
11. `CONTRACTS.md` — locked contracts for every later subagent. See template below.
12. `.claude/settings.local.json` allowlist permitting `cargo *`, `git *`, and general network tooling (curl/wget for non-Nordnet hosts, rustup, apt). The only network rule: **no calls to Nordnet API hosts** (`*.nordnet.*`, `api.test.nordnet.*`, etc.). Reviewer enforces.
13. Pre-commit hook: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.

**Phase 0 gate:**
- All static checks green.
- Auth crypto unit tests pass against fixed RSA test vector.
- Client wiremock tests pass.
- xtask binary builds and `extract-docs --help` works.

### CONTRACTS.md template

```markdown
# Contracts (LOCKED — do not edit after Phase 0)

## Module layout

Every resource group `<group>` owns exactly:
- `crates/nordnet-api/src/models/<group>.rs`
- `crates/nordnet-api/src/resources/<group>.rs`
- `crates/nordnet-api/tests/<group>_test.rs`
- `crates/nordnet-api/fixtures/<group>/*.json`
- `crates/nordnet-api/docs-extract/<group>/*.md`
- `crates/nordnet-cli/src/cmd/<cli_group>.rs` (Phase 4 only; for the `orders` API group, the CLI splits into `cmd/orders_read.rs` + `cmd/orders_write.rs`, dispatched by foundation-owned `cmd/orders.rs`)

No subagent edits files outside its own group.

## Type rules

- All response structs: `#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]`
- All response structs: `#[serde(deny_unknown_fields)]`
- Optional iff doc parameter table marks optional OR doc example shows null. No speculative `Option`.
- Numeric IDs: use newtype from `crate::ids::*`. Never `i64` / `String` raw.
- Timestamps: `time::OffsetDateTime` with `time::serde::iso8601`.
- Money: `crate::models::shared::Money { amount: rust_decimal::Decimal, currency: Currency }`. Never `f64`.
- Enums: full string set from docs, `#[serde(rename_all = "...")]` matching documented casing. Unknown variant = parse error, by design.
- Doc disagreement (parameter table vs example body vs response schema): pick the most-restrictive interpretation, file in `docs-extract/<group>/<op>.md` under "Doc inconsistencies", surface to reviewer.

## Resource function signature

Each operation is a method on `Client`. Naming: `<verb>_<resource>` (`get_account_info`, `place_order`, `cancel_order`, `list_accounts`).

```rust
impl Client {
    pub async fn get_account_info(&self, accid: AccountId) -> Result<AccountInfo, Error>;
}
```

## Test rules

Two test layers per group:

1. **Fixture roundtrip.** For every fixture, `serde_json::from_str::<T>(fixture)` must succeed AND re-serialize must match canonical form.
2. **Wiremock integration.** For every op, mock the endpoint with the fixture as response body, call the resource fn, assert structure matches.

There is **no** "live" test layer. Pipeline never calls the real API.

## Mod files

Never hand-edit `mod.rs`. Run `cargo xtask gen-mods` after adding a new group file. Pre-commit hook calls this and fails if it produces a diff.

## Commit hygiene

- One commit per group (one subagent, one commit), made by Phase 5, not by the implementer.
- Subject: `feat(<group>): implement <ops list>`.
- Commit hook runs full static gate. Failure blocks commit.
```

---

## Phase 1 — Doc extraction (sequential, single agent, sonnet)

One pass through `docs-source/nordnet-api-v2.html`. Slices into per-operation markdown extracts. Per-group output.

**Outputs:**

- `crates/nordnet-api/docs-extract/<group>/<operation>.md` for each non-deprecated op. Contains: HTTP method + path, parameter table (preserved verbatim), request body schema, response body schema, status codes, all `<pre class="example">` example blocks (request + response), and an empty "Doc inconsistencies" section.
- `crates/nordnet-api/docs-extract/INVENTORY.md` — single table mapping `(group, operation, method+path, deprecated?)`. The 2 deprecated ops (`GET /accounts/{accid}`, `GET /news`) are listed and marked `SKIP`.

**Driven by** `cargo xtask extract-docs --html docs-source/nordnet-api-v2.html`. Re-runnable: idempotent over the saved HTML.

**Gate:** human spot-check of inventory against the 44 documented operations (44 documented − 2 deprecated = 42 to implement; the previously-listed `GET /orders/{id}` is absent from the saved HTML, so the orders group has 5 ops). Deprecated ops marked + skipped.

---

## Phase 2 — Fixture assembly (DROPPED)

**Status:** dropped after Phase 1 investigation.

The saved HTML (`docs-source/nordnet-api-v2.html`) is Swagger2Markup output containing schema tables only — no `<pre class="example">` blocks, no JSON example bodies anywhere. Confirmed via direct grep + structural review of the AsciiDoctor output. There is nothing to canonicalize.

**Implementers own their fixtures.** In Phase 3, each group's implementer derives a minimal JSON fixture per op directly from the response schema table in its `docs-extract/<group>/<op>.md` slice, using documented types and example values where the table provides them. Fixtures live in `fixtures/<group>/<op>.{request,response}.json` as before. Each fixture file's first line is a JSON5-style comment-equivalent stored in `fixtures/<group>/<op>.meta.toml`:

```toml
ops = ["get_account_info"]
request_fixture = false
response_fixture = true
fixture_provenance = "synthesized_from_schema"   # or "from_doc_example" if a real one ever surfaces
schema_source = "docs-extract/accounts/get_account_info.md#response-body-schema"
```

This shifts the "fixture realism" burden onto the per-group implementer, who has the schema table loaded already.

---

## Phase 2C — Cross-source consistency check (DROPPED)

**Status:** dropped — degenerate without example bodies.

The check compared parameter table ↔ example request body and response schema ↔ example response body. With no examples, all comparisons would pass vacuously. Removed from the gate sequence.

Cross-endpoint type consistency (Phase 3X) remains and absorbs the safety net: type-name reconciliation across groups stays a hard gate.

---

## Phase 3 — Resource implementation (PARALLEL, sonnet, with reviewer per task)

Big phase. Decompose ~42 ops into 12 API resource groups. One subagent per group. **The API crate has no read/write split — that distinction lives only in the CLI crate (Phase 4).**

### API group decomposition

| Group | Ops |
|---|---|
| `accounts` | list, info, ledgers, positions, returns_today, trades |
| `orders` | list, place, modify, activate, cancel |
| `instruments` | get, lookup, types, types_list, leverages, leverage_filters, suitability, trades, underlyings |
| `instrument_search` | attributes, stocklist, bullbearlist, minifuturelist, optionlist_pairs, unlimitedturbolist |
| `tradables` | info, trades, suitability |
| `markets` | list, get |
| `news` | list (skip deprecated `/news`), news_sources, get_item |
| `tick_sizes` | list, get |
| `countries` | list, get |
| `main_search` | search |
| `login` | start, verify, refresh, logout (surface only; crypto in `auth.rs`) |
| `root` | `GET /api/2` (system info) |

Wave plan: 4 waves of 3 groups (3 implementers + 3 reviewers concurrent), max ~6 subagents at once. Adjust based on observed token/wall-time.

### Implementer prompt template

```
You implement Nordnet API resource group: <GROUP>.

READ ONLY THESE FILES (one parallel batch):
- /Users/alfredvc/src/nordnet-cli/CONTRACTS.md
- /Users/alfredvc/src/nordnet-cli/crates/nordnet-api/docs-extract/<GROUP>/*.md
- /Users/alfredvc/src/nordnet-cli/crates/nordnet-api/fixtures/<GROUP>/*.json
- /Users/alfredvc/src/nordnet-cli/crates/nordnet-api/src/client.rs
- /Users/alfredvc/src/nordnet-cli/crates/nordnet-api/src/error.rs
- /Users/alfredvc/src/nordnet-cli/crates/nordnet-api/src/ids.rs
- /Users/alfredvc/src/nordnet-cli/crates/nordnet-api/src/models/shared.rs
- /Users/alfredvc/src/nordnet-cli/crates/nordnet-api/src/pagination.rs
- /Users/alfredvc/src/nordnet-cli/notes/02C-doc-consistency.md (only entries for <GROUP>)

DO NOT READ:
- Files in any other group's directory.
- The full HTML docs.
- Other groups' models or resources files.

DO NOT CALL THE NORDNET API. Live Nordnet endpoints (`*.nordnet.*`) are off-limits to every agent in this pipeline. General network use (cargo build pulling crates.io, rustup, apt, fetching non-Nordnet docs) is fine. All Nordnet inputs are on local disk under `docs-extract/` and `fixtures/`.

WRITE ONLY THESE FILES:
- crates/nordnet-api/src/models/<GROUP>.rs
- crates/nordnet-api/src/resources/<GROUP>.rs
- crates/nordnet-api/tests/<GROUP>_test.rs

Phase 3 implementers do NOT touch the CLI crate. Phase 4 owns all `crates/nordnet-cli/src/cmd/*.rs` files.

DO NOT WRITE ANY OTHER FILE. No mod.rs edits. No Cargo.toml edits. No shared.rs edits.

If you need a shared type that doesn't exist in shared.rs, return blocked with summary "shared type X needed".

PROCESS:
1. Read inputs.
2. For each fixture, derive Rust type that deserializes it under `deny_unknown_fields`.
3. Reconcile against the parameter table and response schema table in docs-extract. If they disagree with the example body, use the conservative pick from CONTRACTS.md and note it in your notes file.
4. Write models/<GROUP>.rs.
5. Write resources/<GROUP>.rs implementing each operation as a Client method per CONTRACTS.md.
6. Write tests/<GROUP>_test.rs:
   - Roundtrip every fixture.
   - Wiremock integration test for every operation (read AND write — both use the same wiremock pattern).
7. Run, iterating until green:
   - cargo fmt --package nordnet-api
   - cargo clippy --package nordnet-api --tests -- -D warnings
   - cargo test --package nordnet-api --test <GROUP>_test
8. Run `cargo xtask gen-mods` (regenerates mod.rs files; do not edit them yourself).
9. Write notes/3-<GROUP>-impl.md with: ops implemented, doc inconsistencies encountered + how resolved, open questions.

Return JSON:
{
  "status": "done" | "blocked",
  "summary": "<=50 words",
  "files": ["..."],
  "ops_implemented": ["..."]
}

Do NOT git commit. Phase 5 commits.
```

### Reviewer prompt template (one per implementer, parallel-spawned)

```
You review subagent output for Nordnet group: <GROUP>.

READ ONLY:
- /Users/alfredvc/src/nordnet-cli/CONTRACTS.md
- The 4 files written by the implementer (models, resources, tests, cmd).
- The implementer's notes file.
- The fixtures and docs-extract for this group.
- /Users/alfredvc/src/nordnet-cli/notes/02C-doc-consistency.md (entries for this group).

CHECKLIST:
1. Type strictness: every public struct has `#[serde(deny_unknown_fields)]`. No `serde_json::Value` in public API. IDs use `crate::ids::*` newtypes.
2. Optionality: every `Option<T>` justified by doc-marked optional OR null in fixture. Flag speculative options.
3. Enums: variants match documented set exactly. Renames correct.
4. Numeric: no `f64` for money. `Decimal` + `Currency`.
5. Doc-inconsistency handling: every entry in 02C-doc-consistency for this group is addressed in the implementer's notes with a concrete decision matching the conservative-pick rule.
6. Test coverage: every op has wiremock test + every fixture has roundtrip.
7. Lint: re-run `cargo clippy --package nordnet-api --tests -- -D warnings`. Must be clean.
8. Format: re-run `cargo fmt --package nordnet-api --check`. Must be clean.
9. Mod files: implementer did NOT hand-edit any mod.rs. Re-run `cargo xtask gen-mods` and verify `git diff` on mod files is empty.
10. File scope: implementer touched only owned files. `git diff --name-only` against base must match the allowed list exactly.
11. Commit hygiene: no commits made yet.
12. No Nordnet API calls: implementer's notes and code show no attempt to hit `*.nordnet.*` hosts. The crate has no Nordnet hostname in test code (search for `nordnet` host literals; mock URLs like `http://localhost:<port>` and `http://127.0.0.1:<port>` are fine).

If any check fails:
{
  "status": "rework",
  "issues": [{"check": "...", "file": "...", "line": "...", "fix": "..."}],
  "summary": "..."
}

If all pass:
{ "status": "approved", "summary": "..." }
```

### Implementer ↔ reviewer fix loop

- After implementer returns `done`, orchestrator spawns reviewer.
- Reviewer returns `rework` → orchestrator re-dispatches implementer with the issue list. Max 2 rework rounds. Third = pause + escalate to user.
- Reviewer returns `approved` → group marked done in `notes/3-<GROUP>-review.md`.

### Phase 3 gate

- All groups: implementer + reviewer both report `done` / `approved`.
- `cargo test --workspace` green.
- `cargo clippy --workspace --all-targets -- -D warnings` green.

---

## Phase 3X — Cross-endpoint type consistency (sequential, single agent, opus)

After all 12 groups land, types like `Account`, `Instrument`, `Order` may have been independently derived in multiple groups (e.g. `accounts` returns `Account`, `orders` references `Account` in trade fills). Phase 3X reconciles.

**Process:**

1. Build a map: type-name → list of (group, definition snippet) where it appears.
2. For each name with >1 definition: diff the structs.
3. Identical → fine, but redundant. Move to `models/shared.rs` if it's used by ≥3 groups, leave duplicated otherwise (avoiding shared.rs churn).
4. Different → reconcile. Conservative pick: union of fields, with `Option<T>` for any field not present in all definitions. File a note for the reviewer to confirm.

**Gate:** no two groups define a public struct with the same name and different shape. `notes/3X-type-consistency.md` summarizes consolidations.

---

## Phase 4 — CLI surface (PARALLEL, sonnet, reviewer per task)

Subcommands per CLI group. **The CLI splits read and write where applicable; the API does not.** Only `orders` is split into two CLI files; every other group has a single CLI file matching its API group.

### CLI group decomposition (13 groups)

| CLI group | API group | Ops | Subcommand path |
|---|---|---|---|
| `accounts` | `accounts` | list, info, ledgers, positions, returns_today, trades | `nordnet accounts <op>` |
| `orders_read` | `orders` | list | `nordnet orders list` |
| `orders_write` | `orders` | place, modify, activate, cancel | `nordnet orders place / modify / activate / cancel` |
| `instruments` | `instruments` | (all 9) | `nordnet instruments <op>` |
| `instrument_search` | `instrument_search` | (all 6) | `nordnet instrument-search <op>` |
| `tradables` | `tradables` | (all 3) | `nordnet tradables <op>` |
| `markets` | `markets` | list, get | `nordnet markets <op>` |
| `news` | `news` | list, news_sources, get_item | `nordnet news <op>` |
| `tick_sizes` | `tick_sizes` | list, get | `nordnet tick-sizes <op>` |
| `countries` | `countries` | list, get | `nordnet countries <op>` |
| `main_search` | `main_search` | search | `nordnet search <query>` |
| `login` | `login` | start, verify, refresh, logout | `nordnet login <op>` |
| `root` | `root` | system info | `nordnet info` |

`orders_read` and `orders_write` both expose subcommands under the same top-level `nordnet orders ...` namespace; the split is purely a file/commit-ownership split inside the CLI crate, not a UX split.

### Subagent ownership

Each Phase 4 subagent owns exactly one CLI file:
- `crates/nordnet-cli/src/cmd/<CLI_GROUP>.rs`

That's it. Output module is locked. Each subcommand calls one library method on `Client`, runs `output::emit(value, &fields)`. Done.

The `orders` namespace is wired together by a foundation-owned dispatcher `crates/nordnet-cli/src/cmd/orders.rs` (~15 lines, written in Phase 0, locked):

```rust
#[derive(clap::Subcommand)]
pub enum OrdersCmd {
    #[command(flatten)] Read(crate::cmd::orders_read::Cmd),
    #[command(flatten)] Write(crate::cmd::orders_write::Cmd),
}

impl OrdersCmd {
    pub async fn run(self, client: &nordnet_api::Client) -> anyhow::Result<()> {
        match self {
            Self::Read(c) => c.run(client).await,
            Self::Write(c) => c.run(client).await,
        }
    }
}
```

Phase 4 subagents for `orders_read` and `orders_write` each define their own `Cmd` enum and `run` method with no awareness of each other. Zero coupling between the two files.

Wave plan: 5 waves of ~3 groups, max ~6 subagents at once.

**Phase 4 gate:** `cargo run -- <group> <op> --help` works for every op. Wiremock tests at this level optional but encouraged. No calls to Nordnet hosts.

---

## Phase 5 — Workspace integration (sequential, single agent, opus)

The merge step. Conflict-free by design — every implementer wrote in its own files. Only thing here is regenerate mod files and run the full gate, then commit.

**Steps:**

1. `cargo xtask gen-mods` — regenerates all `mod.rs`. Should be a no-op if subagents ran it correctly.
2. `cargo fmt --check` workspace.
3. `cargo clippy --workspace --all-targets -- -D warnings`.
4. `cargo test --workspace`.
5. One git commit per group, in dependency-friendly order: foundation → models → resources → CLI. Each commit triggers pre-commit hook (full gate). Sequential commits, no parallel-write races.
6. Final commit: `chore: regenerate mod files + finalize workspace`.

**Dropped:** per-op CLI wiremock end-to-end smoke. Lib-layer wiremock integration tests (208) already cover URL/method/body/headers/deserialize per op. CLI layer is thin glue (parse args → build request → call → emit); the only unique catch surface is arg-wiring bugs in `cmd/*.rs`, which are caught by type-checked newtype IDs and visual review. Re-asserting "stdout JSON deserializes back into lib type" is near-tautological (tests serde, not our code). Per-op CLI smoke = high ceremony, low marginal value. Dropped per orchestrator decision 2026-05-02.

**No merge conflicts possible** because:

- Foundation files locked after Phase 0.
- Each group's files exclusive to that group's subagent.
- `mod.rs` files generated.
- `Cargo.toml` workspace member list generated by `cargo xtask gen-mods` from `crates/*` listing.

**Phase 5 gate:** all four static gates green. Workspace ready for release. Pipeline ends here.

---

## Conflict elimination — summary

| Risk | Mitigation |
|---|---|
| Two subagents edit same file | File ownership matrix in CONTRACTS.md. Reviewer enforces via `git diff --name-only`. |
| Two subagents edit `mod.rs` | `mod.rs` files generated. Subagents forbidden from editing. Pre-commit hook regenerates and fails on diff. |
| Two subagents add same shared type | Only Phase 0 writes shared types. After Phase 0, `models/shared.rs` is locked. Subagent that needs new shared type returns `blocked` → orchestrator either adds it (rare) or pushes the type into the group's local file (common case). Phase 3X reconciles cross-group duplicates. |
| Cargo.toml conflicts | Workspace `Cargo.toml` only lists `crates/*` (glob). Per-crate `Cargo.toml` for `nordnet-api` and `nordnet-cli` written in Phase 0 with all deps subagents will need; locked thereafter. |
| Parallel git commits race | Phase 5 serializes commits. Subagents do NOT commit. |
| Subagent reads beyond its slice | Prompt template lists exact files to Read. Reviewer can flag if subagent's notes show wider reads. |
| Subagent reaches for the Nordnet API | Allowlist denies Nordnet hosts (`*.nordnet.*`); other network is fine. Reviewer greps for `nordnet` host literals outside doc/fixture paths. CONTRACTS.md states the rule. |

## Correctness rules — universal

1. **Docs are the only source of truth.** Parameter tables, schema tables, and example bodies extracted from `docs-source/nordnet-api-v2.html`. No outside references, no live calls.
2. **`deny_unknown_fields` everywhere** — surfaces drift instantly to whoever runs the binary.
3. **No `f64` for money, ever.** `rust_decimal::Decimal`.
4. **Newtype every ID.** Compile-time prevents passing an `OrderId` where an `AccountId` is expected.
5. **No speculative options.** If a field isn't doc-marked optional and isn't null in any example, it's required.
6. **Doc inconsistencies** are recorded in `docs-extract/<group>/<op>.md` "Doc inconsistencies" section, conservative-picked, surfaced to reviewer.
7. **Cross-source consistency** is a gate, not a hope (Phase 2C).
8. **Cross-endpoint consistency** is a gate, not a hope (Phase 3X).
9. **Pre-commit hook is the floor.** fmt + clippy + test must pass on every commit.
10. **No Nordnet API in pipeline.** Hard-enforced via allowlist (Nordnet hosts blocked); general network allowed for tooling. Documented in CONTRACTS.md; reviewer greps for Nordnet hostnames in code/tests.

## Definition of done

- All ~42 non-deprecated operations implemented and typed.
- Every fixture roundtrips losslessly under `deny_unknown_fields`.
- Every op has a wiremock integration test.
- Cross-endpoint consistency gate passed (Phase 3X); outstanding doc inconsistencies documented. (Phase 2C dropped — see §"Phase 2C".)
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo fmt --check` clean.
- `cargo test --workspace` green.
- Single binary `nordnet` runs `nordnet --help` and prints subcommand tree covering every op.
- README + AGENTS.md generated, both noting that verification is wiremock-only and the live Nordnet API was never contacted by this pipeline.

Real-API verification is the user's responsibility, performed outside this pipeline.

## Locked decisions

1. **Group decomposition.** API crate: 12 groups, no read/write split. CLI crate: 13 groups, with `orders` split into `orders_read` + `orders_write` for separate file ownership and commit. The split is internal to the CLI crate; users still see a single `nordnet orders <op>` namespace.
2. **Write-op exposure in CLI.** Always enabled. No env-var gate, no `--confirm` flag, no hidden-by-default. Standard CLI behavior — irreversible ops are exposed in `--help` and run on invocation. User is responsible for what they invoke.
3. **Locale.** Error messages and CLI text in English.
4. **Branch strategy.** Single branch `ccairgap/misty-octopus-3590`. Phase 5 commits one-per-group sequentially. No worktrees, no per-group PRs.
5. **Saved doc HTML location.** `docs-source/nordnet-api-v2.html` (committed to repo, reproducible from clean checkout).
6. **Auth flow shape (deviation from earlier draft).** Implemented per HTML reference, not the username/password/timestamp variant the earlier process draft described:
   - `POST /login/start` body: `{api_key}`. Response: `{challenge}`.
   - Caller signs `challenge` with their RSA private key.
   - `POST /login/verify` body: `{api_key, service, signature}`. Response carries `session_key`.
   - Auth header: `Authorization: Basic base64(session_key:session_key)`.
   - **Signature scheme: RSA PKCS#1 v1.5 with SHA-256.** Picked because deterministic (testable) and the default for `rsa::pkcs1v15::SigningKey<Sha256>` paired with `ssh-keygen -t rsa`. The HTML only says "signed and base64 encoded challenge string" — the exact scheme lives in an external Getting Started guide not in `docs-source/`. **If the live API rejects this signature, swap `auth::sign_challenge` and re-pin its unit test; structural code is unaffected.** Requires user verification against live login before any real-API run.
7. **Fixture provenance.** HTML contains zero example bodies (Swagger2Markup output). Phase 3 implementers synthesize fixtures from each op's response schema table in `docs-extract/<group>/<op>.md`. Each fixture is paired with `fixtures/<group>/<op>.meta.toml` carrying `fixture_provenance = "synthesized_from_schema"` and `schema_source = "<docs-extract anchor>"`. Reviewer enforces that no fixture is committed without its meta file.
8. **`cmd/orders.rs` feature gate.** The foundation-locked dispatcher is gated behind `feature = "orders-cli"` in `crates/nordnet-cli/Cargo.toml` (off by default) so Phase 0 + Phase 1 builds do not require `cmd/orders_read.rs` and `cmd/orders_write.rs` to exist. **Phase 4 must enable the `orders-cli` feature in `crates/nordnet-cli/Cargo.toml` in the same commit that lands either orders CLI file**, otherwise the dispatcher stays inert and `nordnet orders ...` is missing from the binary.
9. **Phase 0 amendment — `Client::post_form` / `Client::put_form` (added during Phase 3 wave 4, orders group).** Original Phase 0 `Client` only exposed JSON helpers. The Nordnet docs mark every body parameter on `place_order`, `modify_order` (and other write ops we may surface later) as Swagger 2.0 `FormData`, which mandates `application/x-www-form-urlencoded` (no `consumes` directive overrides this in the saved HTML). Two independent reference clients confirm form encoding (the official `nordnet/nordnet-next-api` JS client splits `post`/`postJson`; the `denro/nordnet` Go client hardcodes `Content-Type: application/x-www-form-urlencoded`). JSON would 400 on the live API. Foundation `client.rs` was therefore amended to add `post_form` and `put_form` (using `serde_urlencoded` directly because we build `reqwest` with `default-features = false`). One new error variant `Error::EncodeForm(String)`. Two new wiremock unit tests in `tests/client_test.rs` assert the `application/x-www-form-urlencoded` Content-Type and the urlencoded body. The amendment is otherwise additive — JSON helpers are unchanged.
10. **CONTRACTS carve-out — `Decimal` adapter on form-encoded request types (added during Phase 3 wave 4, orders group).** CONTRACTS.md line 23 mandates `#[serde(with = "rust_decimal::serde::arbitrary_precision")]` on every `Decimal` field. That adapter emits the `$serde_json::private::Number` magic token, which only `serde_json` understands — `serde_urlencoded` rejects it as an unsupported nested struct. Form-encoded request structs (today: `PlaceOrderRequest::price/target_value/trigger_value`, `ModifyOrderRequest::price`) therefore omit the adapter. Default `rust_decimal` serde emits/accepts strings (`"101.5"`), which is lossless and matches the documented FormData wire shape (urlencoded values are always strings). Consequence: request fixtures for these structs carry `Decimal` values as JSON strings, not bare numbers. Response fixtures keep the bare-number convention unchanged. `.meta.toml` files for affected request fixtures record this deviation under a `wire_format = "application/x-www-form-urlencoded"` entry. This carve-out applies only to fields whose owning struct is serialized via `Client::post_form`/`put_form`; any field in a JSON-bodied or response struct still requires the adapter.
11. **Phase 3X amendments — `models/shared.rs` extensions and `crate::ids::IssuerId` promotion (added during Phase 3X, cross-endpoint type consistency).** Phase 3X is the only post-Phase-0 phase permitted to extend `models/shared.rs` and `ids.rs`. The full reconciliation map lives in `notes/3X-type-consistency.md`; the foundation-touching additions are:
    - `crate::ids::IssuerId` — `i64` newtype (`serde(transparent)`). Previously a local newtype `models::instruments::IssuerId`. Promoted because two groups now reference it (`instruments::Issuer.issuer_id`, `instrument_search::InstrumentInfo.issuer_id`); `instrument_search` had been carrying it as plain `i64` for lack of the type.
    - `crate::models::shared::AmountWithCurrency` — `{currency: Currency, value: Decimal}` struct matching the documented `_definitions/Amount.md` schema. Two groups had each declared a local copy (`accounts::Amount` and `orders::OrderAmount`) differing only in whether `currency` was typed as `String` or `Currency`. Both now alias the shared type via `pub use ... as Amount` / `as OrderAmount`, normalizing on the `Currency` newtype. Wire-compatible (`Currency` is `serde(transparent)` over `String`).
    - `crate::models::shared::opt_arb_prec` — serde adapter module for `Option<Decimal>` using arbitrary-precision encoding. Promoted from byte-identical private copies in 4 group files (`accounts`, `instruments`, `instrument_search`, `main_search`).
    - `crate::models::shared::date_iso8601` — serde adapter module for `time::Date` round-tripping the documented `string(date)` (`YYYY-MM-DD`) wire form, with `option` and `vec` submodules for `Option<Date>` / `Vec<Date>`. Used by the Phase 3X switch of `Instrument.expiration_date`, `PositionInstrument.expiration_date`, `AccountInfo.registration_date`, and `LeverageFilter.expiration_dates` from `String` to typed dates.
    - `Tradable.identifier` — type-swap from `String` to `crate::ids::TradableId` (wire-identical). Same applies to a handful of doc-comment touch-ups across the affected group files.

    `models/shared.rs` and `crate::ids` are re-locked after Phase 3X. Phase 4 (CLI) treats them read-only.

12. **Phase 4 amendment — `Cmd::run` signature takes `fields: &[String]` (added at the start of Phase 4, CLI surface).** The Phase 0 locked dispatcher in `crates/nordnet-cli/src/cmd/orders.rs` originally declared `pub async fn run(self, client: &nordnet_api::Client) -> anyhow::Result<()>` — no way for the cmd to access the global `--fields` flag parsed in `main.rs`. Three options were considered: (A) thread `fields: &[String]` through `run`, (B) have cmd functions return `serde_json::Value` and let `main.rs` emit, (C) process-global static. (B) was rejected because returning `Value` forces premature type erasure, blocks future cmd-side typed inspection (logging, multi-emit, branching on response), and adds an extra `serde_json::to_value` allocation per call. (C) was rejected as hidden state. (A) is the standard pattern in Rust clap CLIs (cargo, gh-rs, ripgrep): global flags from the top-level `Cli` are passed down as explicit args. The locked `OrdersCmd::run` signature is therefore now `pub async fn run(self, client: &nordnet_api::Client, fields: &[String]) -> anyhow::Result<()>`, and every Phase 4 cmd file's `Cmd::run` follows the same shape. If a second global flag is added later, callers should refactor to a `CmdContext` struct rather than continue threading individual args.

13. **Phase 4 amendment — `nordnet auth` namespace + on-disk session persistence (added during Phase 4 wave 3, replacing the wave-2 `nordnet login` namespace).** PROCESS.md §"Phase 4 — CLI surface" line 403 originally listed a `login` CLI group with subcommands `start, verify, refresh, logout` mirroring the four `/login*` API ops 1:1. That shape required the user to manually copy `session_key` from `verify`'s JSON output into a `--session-key` flag (or env var) on every subsequent authenticated call — workable but unergonomic and a dead-weight ceremony for every script. The replacement is the `nordnet auth` namespace with four higher-level subcommands: `login` (runs the full `start` → `sign_challenge` → `verify` flow internally and writes the resulting session to disk), `logout` (loads the on-disk session, calls `DELETE /login`, and removes the file), `refresh` (loads, calls `PUT /login`, updates the local `acquired_at` timestamp), and `status` (prints local session metadata without contacting the API). Authenticated commands now load the session transparently — no `--session-key` ceremony required for normal use. The override order, highest-priority first: `--session-key` global flag → `NORDNET_SESSION_KEY` env var → on-disk session file. Foundation additions / changes:
    - New module `crates/nordnet-cli/src/session.rs` — `StoredSession` (TOML-serialized record carrying `session_key`, `expires_in`, `acquired_at`) plus `load`/`save`/`delete` helpers. The default path is `<dirs::config_dir>/nordnet/session.toml` (next to `credentials.toml`). On Unix, `save` sets file mode `0600`; on Windows the OS ACL is used as-is (Unix-first for the primary agent-runtime target).
    - New global flag in `crates/nordnet-cli/src/main.rs`: `--session-key` (with `env = "NORDNET_SESSION_KEY"`). Required adding `clap` workspace feature `"env"`.
    - New workspace dep added to `crates/nordnet-cli/Cargo.toml`: `time = { workspace = true }` (already in workspace deps; needed locally for `OffsetDateTime` and `time::serde::rfc3339`).
    - `crates/nordnet-cli/src/main.rs` gains a `build_client(session_override)` helper that resolves the session per the override order and attaches it to the `Client` for every non-`auth` command.
    - The wave-2 `crates/nordnet-cli/src/cmd/login.rs` is deleted; replaced by `crates/nordnet-cli/src/cmd/auth.rs`. Net surface effect: the user-facing `nordnet login` namespace from wave 2 (`start`, `verify`, `refresh`, `logout`) is removed in favor of the `nordnet auth` namespace.

14. **Phase 4 amendment — `OrdersCmd` gains `Debug` + `#[allow(clippy::large_enum_variant)]`; `orders-cli` feature on by default (added during Phase 4 wave 4).** Wiring `cmd::orders::OrdersCmd` into the top-level `Command` enum (which derives `Debug`) required the locked dispatcher's enum to derive `Debug` too. Also, clippy's `large_enum_variant` lint fired because `OrdersCmd::Write` carries `orders_write::Cmd::Place(PlaceArgs)` whose 15 fields put the variant at ~240 bytes (vs `Read` ~16). Boxing `Write(Box<orders_write::Cmd>)` doesn't compose with clap's `#[command(flatten)]` derive plumbing, and the size only matters for runtime perf — the enum is constructed exactly once per CLI invocation, so an indirection would buy nothing. Lint allowed at the dispatcher with an inline comment explaining the rationale. The `orders-cli` feature flipped from `default = []` to `default = ["orders-cli"]` because both `orders_read.rs` and `orders_write.rs` now exist (wave 4 wave_4 implementers landed them together). The feature gate is retained because the dispatcher contract still requires a guard against the brief Phase 0 → Phase 4 window when only the dispatcher existed.

## Pipeline state log

| Phase | Status | Commit | Notes |
|---|---|---|---|
| 0 Foundation | done | `ccbcd05` | 39 tests green; auth deviation logged in §Locked decisions #6 |
| 1 Doc extraction | done | `1a50c7d` | 43 op extracts + INVENTORY.md; 5 orders ops (no `get`) |
| 1.5 Definitions extraction | done | `0a25c3d` | 78 type definitions extracted from Swagger2Markup Definitions section; per-op files cross-link to types |
| 2 Fixture assembly | dropped | — | HTML has no example bodies; deferred to Phase 3 implementers |
| 2C Cross-source consistency | dropped | — | Degenerate without examples |
| Decimal serde fix | done | `6412c1b` | All Decimal fields use `#[serde(with = "rust_decimal::serde::arbitrary_precision")]`; bare-number JSON form |
| 3 wave 1 (root, countries, tick_sizes) | done | `e160d95`, `4c2a5d9`, `6b712c4` | 58 tests green; 5 ops |
| 3 wave 2 (markets, main_search, news) | done | `ae7142f`, `9fab32b`, `427edf6` | 6 ops |
| 3 wave 3 (login, tradables, instruments) | done | `26df040`+`01eaad0`, `d9b3a7f`, `e6dc8e8` | 16 ops; `put_empty` added for body-less PUT |
| 3 wave 4 (accounts, instrument_search, orders) | done | `51611eb`, `746493d`, `6478a50`, `589498b` | 17 ops; foundation amended with `post_form`/`put_form` (item 9) and Decimal carve-out (item 10); 258 tests workspace-wide |
| 3X Cross-endpoint consistency | done | `73dc33c` | 264 tests green (+6); 4 foundation additions to `shared.rs`/`ids.rs` (item 11); reconciliation map in `notes/3X-type-consistency.md` |
| 4 CLI surface | done | `6de6ae5`, `f35c116`, `92bb151`, `a955e5c`, `3775fe1` | 13 CLI groups, 270 tests green (+6 session); foundation amendments items 12, 13, 14 |
| 5 Workspace integration | done | (doc-only commit) | gen-mods no-op; fmt clean; clippy -D warnings clean; `cargo test --workspace` 270 green. Step 5 (per-op CLI wiremock smoke) dropped — see §"Phase 5". Pipeline complete. |

## Post-Phase-5 hardening

Targeted fixes after pipeline closed. Each entry is a standalone commit, not part of a phase.

| Change | Commit | Notes |
|---|---|---|
| `client`: drop auto-retry on 429/503 | `039a04c` | Caller decides backoff; client returns `Error::RateLimited`/`Error::ServiceUnavailable` directly. |
| `cli`: atomic 0600 session save | `58d807f` | Closes secret-leak race when `~/.nordnet/session.toml` is rewritten under concurrent reads. |
| `auth`: refresh-without-`expires_in` assumption | `962df91` | Doc-only — records that refresh response may omit `expires_in`; we keep the previous TTL. |
| `tradables::OrderType` → `AllowedOrderType` | `fc0e327` | Removes name collision with `orders::OrderType` (closed request enum). The tradables type is the per-instrument `(name, type)` capability pair; the orders type is the wire enum. WIP rename was carried in via merge `1d05ac3`; this commit completes it (fmt fix on import order in `tests/tradables_test.rs`) and documents the change. |
| `auth`: RSA → Ed25519 (real signature scheme) | (this commit) | The pipeline shipped with PKCS#1 v1.5 + SHA-256 + an `auth=encrypted_creds` field, which targeted a deprecated pre-2018 endpoint. The current Nordnet External API v2 uses pure **Ed25519** over the raw UTF-8 bytes of the challenge, with **no** encrypted-credentials field — verified against `nordnet/next-api-v2-examples/python3/sign.py` (Sep 2025) and the official "Getting Started" docs. Workspace deps swap `rsa`+`sha2`+`rand_chacha` for `ed25519-dalek`+`ssh-key`. Keys are loaded from OpenSSH on-disk format (`ssh-keygen -t ed25519`). Replaces Locked Decision #6's "we guessed PKCS#1 v1.5 + SHA-256" rationale. |
| `instrument_search`: query CSV not multi-key | (this commit) | Multi-value query params (`attribute_group`, `attributes`, `expand`, `minmax`) now serialize as a single comma-separated value (`?x=A,B,C`), Swagger 2.0 `collectionFormat=csv` — verified against Nordnet's own JS client `nordnet/nordnet-next-api` (`uriEncode` does `value.join(',')`). Previous code emitted repeated keys (`?x=A&x=B`), which the live API would silently treat as a single-element query (last value wins under most servers' multimap parsers). |
| Models: drop `deny_unknown_fields` on response types | (this commit) | All response structs now silently ignore undocumented server fields. The doc-extract schemas are not guaranteed exhaustive vs the live API — a single extra field would otherwise surface as `Error::Decode` and kill every read call. Request structs (`PlaceOrderRequest`, `ModifyOrderRequest`, `ApiKeyStartLoginRequest`, `ApiKeyVerifyLoginRequest`) keep `deny_unknown_fields` to catch our own bugs at compile/test time. 26 `*_rejects_unknown_fields` tests removed; one tolerance assertion may be added later if useful. |

## Locked decision update — #6 (signature scheme)

The Phase 0 "Locked decision #6" recorded RSA PKCS#1 v1.5 + SHA-256 as
the signature scheme based on the published HTML doc page being silent
on the algorithm. The follow-up research found Nordnet's own current
example repo (`nordnet/next-api-v2-examples`, last commit Sep 2025) and
the public "Getting Started" doc both specify Ed25519 with raw signing
and no namespace. The "RSA + encrypted-credentials" flow that older
community clients (`larssonandreas`, `denro/nordnet`, `fhqvst/nordnet`)
implement is for a deprecated pre-2018 endpoint that no longer exists.
The new auth.rs uses pure Ed25519 over the raw UTF-8 bytes of the
challenge per the official Python `sign.py` reference. No live test
yet — first authenticated call will confirm.
