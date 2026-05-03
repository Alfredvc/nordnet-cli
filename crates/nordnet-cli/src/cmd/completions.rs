//! `nordnet completions <shell>` — emit a shell completion script to stdout.
//!
//! Runtime generation pattern (used by starship, jj, etc.) — survives
//! `cargo install` because it does not rely on `build.rs` artifacts that
//! are discarded after install. Pipe the output into your shell's
//! completion directory.
//!
//! # Examples
//!
//! ```sh
//! # Bash (Linux)
//! nordnet completions bash > ~/.local/share/bash-completion/completions/nordnet
//!
//! # Zsh — append to a directory in $fpath
//! nordnet completions zsh > "${fpath[1]}/_nordnet"
//!
//! # Fish
//! nordnet completions fish > ~/.config/fish/completions/nordnet.fish
//! ```

use clap::{Args, CommandFactory};
use clap_complete::{generate, Shell};

/// Arguments for `nordnet completions`.
#[derive(Debug, Args)]
pub struct Cmd {
    /// Target shell. One of: `bash`, `zsh`, `fish`, `powershell`, `elvish`.
    pub shell: Shell,
}

impl Cmd {
    pub fn run(self) {
        let mut cmd = crate::Cli::command();
        generate(self.shell, &mut cmd, "nordnet", &mut std::io::stdout());
    }
}
