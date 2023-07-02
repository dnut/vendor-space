use std::{
    ffi::OsString,
    fs::canonicalize,
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
    pub repos: Vec<Repo>,
}

#[derive(Debug)]
pub struct Repo {
    pub name: String,
    pub url: String,
    pub branches: Vec<String>,
    pub allow_existing: bool,
}

pub fn load_config() -> Result<Config, ConfigError> {
    let args = Args::parse()?;
    let allow_existing_override = args.allow_existing_override();
    let config_file_path = match args.config {
        Some(path_string) => canonicalize(path_string)?,
        None => {
            let cli_root = canonicalize(args.root.clone().unwrap_or(".".into()))?;
            find_file_up(&cli_root, "vendor-space.toml")?
        }
    };
    let config_file_string = std::fs::read_to_string(&config_file_path)?;
    let config_file = parse_config_file(&config_file_string)?;

    let root = match args.root {
        Some(root) => canonicalize(root)?,
        None => {
            let root = match config_file.header.root {
                Some(root) => PathBuf::from(root),
                None => PathBuf::from("."),
            };
            if root.is_absolute() {
                canonicalize(root)?
            } else {
                let config_file_dir = config_file_path.parent().ok_or(ConfigError::Unexpected(
                    "Failed to identify the parent directory of the config file.".into(),
                ))?;
                config_file_dir.join(root)
            }
        }
    }
    .into_os_string()
    .into_string()
    .map_err(ConfigError::OsStringConversion)?;

    let mut repos = vec![];
    for repo in config_file.repos {
        repos.push(Repo {
            name: repo.name,
            url: repo.url,
            branches: repo.branches.unwrap_or(vec!["master".into()]),
            allow_existing: match &allow_existing_override {
                Some(allow_existing) => *allow_existing,
                None => repo.allow_existing.unwrap_or(
                    config_file
                        .header
                        .allow_existing
                        .unwrap_or(ALLOW_EXISTING_DEFAULT),
                ),
            },
        });
    }

    Ok(Config { root, repos })
}

/// Prepare a vendor space from a config file.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Filesystem path to the config file.
    ///
    /// Defaults to the nearest vendor-space.toml in the current or a parent directory
    #[arg(short, long)]
    config: Option<String>,

    /// Root folder, within which the vendor space will be created.
    ///
    /// Defaults to the value in the config file, which defaults to the
    /// directory where the config file is located.
    #[arg(short, long)]
    root: Option<String>,

    /// Overrides all values in the config file to allow usage of any existing repos.
    #[arg(short, long)]
    allow_existing: bool,

    /// Overrides all values in the config file to fail to use any existing repos.
    #[arg(short, long)]
    block_existing: bool,
}

impl Args {
    fn parse() -> Result<Self, ConfigError> {
        let this = <Self as Parser>::parse();
        if this.allow_existing && this.block_existing {
            return Err(ConfigError::InvalidInput(
                "May not both allow and block existing".into(),
            ));
        }
        Ok(this)
    }

    fn allow_existing_override(&self) -> Option<bool> {
        if self.allow_existing && self.block_existing {
            bug!("cli args were not validated");
        }
        if self.allow_existing || self.block_existing {
            Some(self.allow_existing && !self.block_existing)
        } else {
            None
        }
    }
}

fn find_file_up(start_directory: &Path, name: &str) -> Result<PathBuf, ConfigError> {
    for result in std::fs::read_dir(start_directory)? {
        let item = result?;
        if item.file_name() == name && item.file_type()?.is_file() {
            return Ok(item.path());
        }
    }
    Err(ConfigError::InvalidInput(
        "{name} not found in {start_directory} or its parents".into(),
    ))
}

fn parse_config_file(toml_string: &str) -> Result<ConfigFile, ConfigError> {
    let table = toml_string.parse::<Table>()?;
    Ok(ConfigFile {
        header: toml::from_str(&toml_string)?,
        repos: table
            .into_iter()
            .filter_map(|(name, item)| match item {
                toml::Value::Table(mut table) => {
                    table.insert("name".into(), Value::String(name));
                    table.try_into().ok()
                }
                _ => None,
            })
            .collect(),
    })
}

/// Simply represents the parsed data in the config file without any special
/// logic applied to transform the data or set defaults.
///
/// Custom logic will compare this data to information from the command line and
/// the filesystem when determining the actual application configuration. These
/// different pieces of logic are kept separate to make the code maintainable
/// and easy to reason about. Likewise, this data structure should only ever
/// represent the exact deserialized state of the config file. It is important
/// for callers to distinguish between explicitly set values versus default
/// values.
///
/// Optional fields should be represented with Options. No data from one field
/// should be used to imply the state of another field. No filesystem paths
/// should be represented as anything other than the provided string.
#[derive(Debug)]
struct ConfigFile {
    header: ConfigFileHeader,
    repos: Vec<ConfigFileRepo>,
}

/// Free floating fields at the top level of the config file. See docs for
/// `ConfigFile`
#[derive(Debug, Deserialize)]
struct ConfigFileHeader {
    root: Option<String>,
    allow_existing: Option<bool>,
}

/// A section within the config file that represents one of the repositories to
/// manage. See docs for `ConfigFile`
#[derive(Debug, Deserialize)]
struct ConfigFileRepo {
    name: String,
    url: String,
    branches: Option<Vec<String>>,
    allow_existing: Option<bool>,
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

/// weaker alternative to `unreachable` where it is not as easily proven that
/// the code is truly unreachable.
macro_rules! bug {
    ($($arg:tt)*) => {
        panic!("encountered a bug that requires a code change to fix it: {}", format!($($arg)*))
    };
}
use bug;
