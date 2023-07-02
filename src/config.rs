use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use clap::Parser;
use serde::Deserialize;
use toml::{Table, Value};

/// todo put some more thought into this
/// - errors may be surprising but easy to correct
/// - silently using existing code when not expected could be a problem
const ALLOW_EXISTING_DEFAULT: bool = false;

#[derive(Debug)]
pub struct Config {
    /// A canonical absolute path, formatted as an OS string for the current
    /// operating system, indicating where the repositories should be.
    ///
    /// When the root is configured in the `SOURCE` as a relative path, it is
    /// relative to the `PARENT`. It must be converted to an absolute path
    /// before being stored in this struct.
    /// - if SOURCE = "config file", PARENT = "config file's parent directory"
    /// - if SOURCE = "command line", PARENT = "current working directory"
    pub root: String,
    pub repos: Vec<RepoConfig>,
}

#[derive(Debug, Deserialize)]
pub struct RepoConfig {
    pub name: String,

    pub url: String,

    #[serde(default = "master")]
    pub branches: Vec<String>,

    #[serde(default)]
    pub allow_existing: bool,
}

/// Prepare a vendor space from a config file.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Filesystem path to the config file.
    #[arg(short, long, default_value = "vendor-space.toml")]
    config: String,

    /// Root folder, within which the vendor-space will be created.
    ///
    /// Defaults to the value in the config file, which defaults to the
    /// directory where the config file is located.
    #[arg(short, long)]
    root: Option<String>,

    // todo Overrides all values in the config file? Or should it only override the
    // top level setting?
    /// not available
    #[arg(short, long)]
    allow_existing: bool,

    // todo is this the right approach?
    /// not available
    #[arg(short, long)]
    block_existing: bool,
}

pub fn load_config() -> Result<Config, ConfigError> {
    let args = Args::parse();
    if args.allow_existing && args.block_existing {
        return Err(ConfigError::InvalidInput(
            "you may not set both --allow-existing and --block-existing".into(),
        ));
    }
    if args.allow_existing || args.block_existing {
        unimplemented!("haven't decided how to use allow or block cli options");
    }
    let config_file_path = std::fs::canonicalize(&args.config)?;
    let contents = std::fs::read_to_string(&config_file_path)?;
    let mut config = parse_config(
        &contents,
        config_file_path
            .parent()
            .ok_or(ConfigError::Unexpected(
                "Failed to identify the parent directory of the config file.".into(),
            ))?
            .to_path_buf(),
    )?;
    if let Some(root) = args.root {
        config.root = canonicalize_to_string(root)?;
    }

    Ok(config)
}

fn parse_config(toml_string: &str, parent_directory: PathBuf) -> Result<Config, ConfigError> {
    let table = toml_string.parse::<Table>()?;
    let mut root: PathBuf = table["root"].as_str().unwrap_or(".").into();
    if root.is_relative() {
        root = parent_directory.join(root);
    }
    let allow_existing = table
        .get("allow-existing")
        .map(|x| {
            x.as_bool().ok_or(ConfigError::InvalidInput(
                "allow-existing must be a bool".into(),
            ))
        })
        .unwrap_or(Ok(ALLOW_EXISTING_DEFAULT))?;
    Ok(Config {
        root: canonicalize_to_string(&root)?,
        repos: table
            .into_iter()
            .filter_map(|(name, item)| match item {
                toml::Value::Table(mut table) => {
                    table.insert("name".into(), Value::String(name));
                    if let toml::map::Entry::Vacant(x) = table.entry("allow-existing") {
                        x.insert(Value::Boolean(allow_existing));
                    }
                    table.try_into().ok()
                }
                _ => None,
            })
            .collect(),
    })
}

fn canonicalize_to_string<P: AsRef<Path>>(path: P) -> Result<String, ConfigError> {
    std::fs::canonicalize(path)?
        .into_os_string()
        .into_string()
        .map_err(ConfigError::OsStringConversion)
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("The provided configuration was invalid: {0}")]
    InvalidInput(String),

    #[error("Got an unexpected error: {0}")]
    Unexpected(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to deserialize toml: {0}")]
    TomlDeserialization(#[from] toml::de::Error),

    #[error("failed to convert OsString '{0:?}' to String")]
    OsStringConversion(OsString),
}

fn master() -> Vec<String> {
    vec!["master".into()]
}
