//! Workspace task runner.
//!
//! - `gen-mods` — regenerates every managed `mod.rs` from the filesystem.
//!
//! Run with: `cargo run -p xtask -- gen-mods`.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};

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
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::GenMods => gen_mods(),
    }
}

/// The set of directories whose `mod.rs` we manage. Each entry is rooted
/// at the workspace root.
const MOD_DIRS: &[&str] = &[
    "crates/nordnet-api/src/resources",
    "crates/nordnet-cli/src/cmd",
    "crates/nordnet-model/src/models",
];

fn gen_mods() -> Result<()> {
    let workspace_root = workspace_root()?;
    for rel in MOD_DIRS {
        let dir = workspace_root.join(rel);
        if !dir.exists() {
            continue;
        }
        let body = build_mod_body(&dir).with_context(|| format!("scanning {}", dir.display()))?;
        let mod_path = dir.join("mod.rs");
        let new_contents = format!(
            "// GENERATED — do not hand-edit. Regenerate with `cargo xtask gen-mods`.\n{body}"
        );
        write_if_changed(&mod_path, &new_contents)?;
    }
    Ok(())
}

fn build_mod_body(dir: &Path) -> Result<String> {
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
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .context("CARGO_MANIFEST_DIR not set; run via `cargo run -p xtask`")?;
    let p = PathBuf::from(manifest);
    p.parent()
        .map(Path::to_path_buf)
        .context("xtask manifest dir has no parent")
}
