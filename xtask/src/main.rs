//! Workspace task runner. Three subcommands (per PROCESS.md §"Phase 0"):
//!
//! - `gen-mods` — regenerates every `mod.rs` from the filesystem.
//! - `extract-docs --html <path>` — re-extract per-op docs + fixtures.
//! - `consistency-check` — cross-source + cross-endpoint checker.
//!   Stub in Phase 0; implemented by Phase 2C / 3X.
//!
//! Run with: `cargo run -p xtask -- <subcommand>`.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};

mod extract_docs;

#[derive(Debug, Parser)]
#[command(name = "xtask", version, about = "Nordnet CLI workspace tasks.")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Regenerate every `mod.rs` under managed directories from the
    /// filesystem. Idempotent.
    GenMods,

    /// Re-extract per-operation docs + fixtures from the saved HTML.
    ExtractDocs {
        /// Path to the saved Nordnet API reference HTML.
        #[arg(long)]
        html: PathBuf,
    },

    /// Cross-source + cross-endpoint doc consistency report. Stub in
    /// Phase 0 — full implementation lands in Phase 2C / 3X.
    ConsistencyCheck,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::GenMods => gen_mods(),
        Cmd::ExtractDocs { html } => extract_docs::run(&html),
        Cmd::ConsistencyCheck => {
            eprintln!(
                "consistency-check is not yet implemented (Phase 0 stub). \
                 Will run cross-source + cross-endpoint checks in Phase 2C / 3X."
            );
            Ok(())
        }
    }
}

/// The set of directories whose `mod.rs` we manage. Each entry is rooted
/// at the workspace root.
const MOD_DIRS: &[(&str, ManagedKind)] = &[
    ("crates/nordnet-api/src/models", ManagedKind::ModelsApi),
    ("crates/nordnet-api/src/resources", ManagedKind::PlainPub),
    ("crates/nordnet-cli/src/cmd", ManagedKind::PlainPub),
];

#[derive(Clone, Copy)]
enum ManagedKind {
    /// `models/`: `shared` is special — it must always be declared even
    /// if other group files come and go.
    ModelsApi,
    /// `resources/` and `cmd/`: just `pub mod <stem>;` for every `*.rs`
    /// in the directory other than `mod.rs`.
    PlainPub,
}

fn gen_mods() -> Result<()> {
    let workspace_root = workspace_root()?;
    for (rel, kind) in MOD_DIRS {
        let dir = workspace_root.join(rel);
        if !dir.exists() {
            // Optional directory — skip silently. Phase 3 implementers
            // create per-group files under these directories.
            continue;
        }
        let body =
            build_mod_body(&dir, *kind).with_context(|| format!("scanning {}", dir.display()))?;
        let mod_path = dir.join("mod.rs");
        let new_contents = format!(
            "// GENERATED — do not hand-edit. Regenerate with `cargo xtask gen-mods`.\n{body}"
        );
        write_if_changed(&mod_path, &new_contents)?;
    }
    Ok(())
}

fn build_mod_body(dir: &Path, kind: ManagedKind) -> Result<String> {
    let mut stems: Vec<String> = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_owned())
            .unwrap_or_default();
        if stem.is_empty() || stem == "mod" {
            continue;
        }
        stems.push(stem);
    }
    stems.sort();
    let mut body = String::new();
    for stem in &stems {
        body.push_str(&format!("pub mod {stem};\n"));
    }
    let _ = kind; // reserved for future per-kind decoration (e.g. re-exports)
    Ok(body)
}

fn write_if_changed(path: &Path, contents: &str) -> Result<()> {
    if let Ok(existing) = fs::read_to_string(path) {
        if existing == contents {
            return Ok(());
        }
    }
    fs::write(path, contents).with_context(|| format!("writing {}", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}

fn workspace_root() -> Result<PathBuf> {
    // We assume `cargo run -p xtask` is launched from the workspace
    // root. Cargo sets `CARGO_MANIFEST_DIR` to the xtask crate; its
    // parent is the workspace root.
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .context("CARGO_MANIFEST_DIR not set; run via `cargo run -p xtask`")?;
    let p = PathBuf::from(manifest);
    p.parent()
        .map(Path::to_path_buf)
        .context("xtask manifest dir has no parent")
}
