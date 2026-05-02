//! Credential and config loading.
//!
//! Resolution order (first hit wins per field):
//! 1. Environment variables:
//!    - `NORDNET_API_KEY` -> `api_key`
//!    - `NORDNET_SERVICE` -> `service` (defaults to `"NEXTAPI"`)
//!    - `NORDNET_KEY_PATH` -> `key_path`
//!    - `NORDNET_DEFAULT_ACCOUNT` -> `default_account`
//!    - `NORDNET_BASE_URL` -> `base_url` (defaults to public production URL)
//! 2. `~/.config/nordnet/credentials.toml` (overridable via
//!    `$XDG_CONFIG_HOME` per `dirs::config_dir`):
//!
//! ```toml
//! api_key = "..."
//! service = "NEXTAPI"
//! key_path = "/path/to/id_ed25519"  # OpenSSH-format Ed25519 private key
//! default_account = 1234567
//! base_url = "https://public.nordnet.se/api/2"
//! ```
//!
//! Missing required fields surface as [`ConfigError::Missing`]. The
//! caller decides whether a missing field is fatal — for read-only ops
//! that target a public URL, only `base_url` may be required.

use serde::Deserialize;
use std::path::{Path, PathBuf};
use thiserror::Error;

const DEFAULT_BASE_URL: &str = "https://public.nordnet.se/api/2";
const DEFAULT_SERVICE: &str = "NEXTAPI";

/// Resolved credentials + connection settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub base_url: String,
    pub api_key: Option<String>,
    pub service: String,
    pub key_path: Option<PathBuf>,
    pub default_account: Option<i64>,
}

impl Config {
    /// Build a config from defaults only (no env, no file). Useful for
    /// tests that drive the CLI against `wiremock`.
    pub fn defaults() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_owned(),
            api_key: None,
            service: DEFAULT_SERVICE.to_owned(),
            key_path: None,
            default_account: None,
        }
    }

    /// Return the [`PathBuf`] where the credentials file is expected.
    /// Returns `None` if no config home can be determined.
    pub fn default_credentials_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("nordnet").join("credentials.toml"))
    }

    /// Load config from environment + the default credentials path.
    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::default_credentials_path();
        Self::load_from(path.as_deref(), &std::env::vars().collect::<Vec<_>>())
    }

    /// Load with an explicit path + an environment slice. Pure function:
    /// no I/O beyond reading the file at `file`.
    pub fn load_from(file: Option<&Path>, env: &[(String, String)]) -> Result<Self, ConfigError> {
        let mut cfg = Self::defaults();

        // File first (lower priority than env).
        if let Some(path) = file {
            if path.exists() {
                let raw = std::fs::read_to_string(path)
                    .map_err(|e| ConfigError::Io(path.to_path_buf(), e))?;
                let file_cfg: FileConfig = toml::from_str(&raw)
                    .map_err(|e| ConfigError::Toml(path.to_path_buf(), e.to_string()))?;
                if let Some(v) = file_cfg.base_url {
                    cfg.base_url = v;
                }
                if let Some(v) = file_cfg.api_key {
                    cfg.api_key = Some(v);
                }
                if let Some(v) = file_cfg.service {
                    cfg.service = v;
                }
                if let Some(v) = file_cfg.key_path {
                    cfg.key_path = Some(PathBuf::from(v));
                }
                if let Some(v) = file_cfg.default_account {
                    cfg.default_account = Some(v);
                }
            }
        }

        // Env overrides file.
        let env_get = |k: &str| env.iter().find(|(ek, _)| ek == k).map(|(_, v)| v.clone());
        if let Some(v) = env_get("NORDNET_BASE_URL") {
            cfg.base_url = v;
        }
        if let Some(v) = env_get("NORDNET_API_KEY") {
            cfg.api_key = Some(v);
        }
        if let Some(v) = env_get("NORDNET_SERVICE") {
            cfg.service = v;
        }
        if let Some(v) = env_get("NORDNET_KEY_PATH") {
            cfg.key_path = Some(PathBuf::from(v));
        }
        if let Some(v) = env_get("NORDNET_DEFAULT_ACCOUNT") {
            cfg.default_account = Some(
                v.parse::<i64>()
                    .map_err(|_| ConfigError::Invalid("NORDNET_DEFAULT_ACCOUNT", v))?,
            );
        }

        Ok(cfg)
    }

    /// Returns the api_key, or [`ConfigError::Missing`] if not set.
    /// Used by Phase 4 commands that require authentication.
    #[allow(dead_code)]
    pub fn require_api_key(&self) -> Result<&str, ConfigError> {
        self.api_key
            .as_deref()
            .ok_or(ConfigError::Missing("api_key"))
    }

    /// Returns the key_path, or [`ConfigError::Missing`] if not set.
    /// Used by Phase 4 commands that require authentication.
    #[allow(dead_code)]
    pub fn require_key_path(&self) -> Result<&Path, ConfigError> {
        self.key_path
            .as_deref()
            .ok_or(ConfigError::Missing("key_path"))
    }
}

#[derive(Debug, Deserialize, Default)]
struct FileConfig {
    base_url: Option<String>,
    api_key: Option<String>,
    service: Option<String>,
    key_path: Option<String>,
    default_account: Option<i64>,
}

#[derive(Debug, Error)]
#[allow(dead_code)] // `Missing` is constructed by Phase 4 require_* helpers above.
pub enum ConfigError {
    #[error("missing required configuration field: {0}")]
    Missing(&'static str),
    #[error("invalid value for {0}: {1}")]
    Invalid(&'static str, String),
    #[error("could not read {0}: {1}")]
    Io(PathBuf, std::io::Error),
    #[error("could not parse {0} as TOML: {1}")]
    Toml(PathBuf, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_have_sane_values() {
        let c = Config::defaults();
        assert_eq!(c.base_url, DEFAULT_BASE_URL);
        assert_eq!(c.service, DEFAULT_SERVICE);
        assert!(c.api_key.is_none());
    }

    #[test]
    fn env_overrides_default() {
        let env = vec![
            ("NORDNET_API_KEY".to_string(), "k".to_string()),
            ("NORDNET_BASE_URL".to_string(), "http://x".to_string()),
            ("NORDNET_DEFAULT_ACCOUNT".to_string(), "42".to_string()),
        ];
        let c = Config::load_from(None, &env).unwrap();
        assert_eq!(c.api_key.as_deref(), Some("k"));
        assert_eq!(c.base_url, "http://x");
        assert_eq!(c.default_account, Some(42));
    }

    #[test]
    fn invalid_default_account_errors() {
        let env = vec![(
            "NORDNET_DEFAULT_ACCOUNT".to_string(),
            "not-an-int".to_string(),
        )];
        let r = Config::load_from(None, &env);
        assert!(matches!(
            r,
            Err(ConfigError::Invalid("NORDNET_DEFAULT_ACCOUNT", _))
        ));
    }

    #[test]
    fn file_then_env_layered() {
        let dir = tempdir();
        let path = dir.join("credentials.toml");
        std::fs::write(
            &path,
            r#"
api_key = "from-file"
service = "FILESERVICE"
default_account = 7
"#,
        )
        .unwrap();
        let env = vec![("NORDNET_API_KEY".to_string(), "from-env".to_string())];
        let c = Config::load_from(Some(&path), &env).unwrap();
        // env wins for api_key
        assert_eq!(c.api_key.as_deref(), Some("from-env"));
        // file wins for unset env values
        assert_eq!(c.service, "FILESERVICE");
        assert_eq!(c.default_account, Some(7));
    }

    fn tempdir() -> PathBuf {
        // No external tempdir crate — use a per-test directory under the
        // OS temp root, named with the test thread ID to avoid races.
        let base = std::env::temp_dir().join(format!(
            "nordnet-cli-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&base).unwrap();
        base
    }
}
