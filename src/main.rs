use cmd::ShellError;
use config::{load_config, ConfigError};

pub mod cmd;
pub mod config;

fn main() -> Result<(), VendorSpaceError> {
    let config = load_config()?;

    safe_bash!("mkdir -p {}", config.root)?;

    for repo in config.repos {
        let path = format!("{}/{}", config.root, repo.name);
        get_code(&path, &repo.url, repo.allow_existing)?;
        vendor(&path, &repo.branches)?;
    }

    Ok(())
}

/// todo improve the logic for allow_existing
fn get_code(path: &str, url: &str, allow_existing: bool) -> Result<(), ShellError> {
    let result = safe_bash!("git clone {url} {path}");
    if result.is_err() && allow_existing {
        println!("Ignoring git clone error since allow-existing is enabled");
        Ok(())
    } else {
        result
    }
}

fn vendor(path: &str, branches: &[String]) -> Result<(), ShellError> {
    let first_branch = &branches[0];
    make_bash!(my_bash: "set -euxo pipefail && cd {path}");
    my_bash! {"
        mkdir -p .cargo
        git checkout {first_branch}
        cargo vendor --versioned-dirs 1> .cargo/{first_branch}.config.toml
        cp .cargo/{first_branch}.config.toml .cargo/config.toml
    "}?;
    for branch in branches.iter().skip(1) {
        my_bash! {"
            git checkout {branch}
            git pull
            cargo vendor --no-delete --versioned-dirs 1> .cargo/{branch}.config.toml
        "}?;
    }
    my_bash! {"git checkout {first_branch}"}?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum VendorSpaceError {
    #[error("{0}")]
    Config(#[from] ConfigError),

    #[error("{0}")]
    Shell(#[from] ShellError),
}
