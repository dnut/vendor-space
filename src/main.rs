use std::path::Path;

use cmd::ShellError;
use config::{load_config, ConfigError};

use crate::cmd::CommandExt;

pub mod cmd;
pub mod config;

fn main() -> Result<(), VendorSpaceError> {
    let config = load_config()?;

    safe_bash!("mkdir -p {}", config.root).play()?;

    for repo in config.repos {
        let path = format!("{}/{}", config.root, repo.name);
        get_code(&path, &repo.url, repo.allow_existing)?;
        vendor(&path, &repo.branches)?;
    }

    Ok(())
}

// todo allow working changes if the only branch we want to use is the already
// checked out branch
fn get_code(path: &str, url: &str, allow_existing: bool) -> Result<(), RepositoryError> {
    let typed_path = Path::new(path);
    if typed_path.is_file() {
        Err(RepositoryError::PathIsFile(path.into()))
    } else if typed_path.is_dir() {
        if allow_existing {
            if !typed_path.join(Path::new(".git")).is_dir()
                || safe_bash!("cd {path} && git status").play().is_err()
            {
                return Err(RepositoryError::InvalidGitRepo(path.into()));
            }
            safe_bash!("cd {path} && git diff-index --quiet HEAD --")
                .play()
                .map_err(|_| RepositoryError::DirtyWorkingTree(path.into()))
        } else {
            Err(RepositoryError::DirectoryAlreadyExists(path.into()))
        }
    } else {
        safe_bash!("git clone {url} {path}")
            .play()
            .map_err(RepositoryError::GitClone)
    }
}

fn vendor(path: &str, branches: &[String]) -> Result<(), ShellError> {
    let first_branch = &branches[0];
    make_bash!(my_bash: "set -euxo pipefail && cd {path}");
    my_bash! {"
        mkdir -p .cargo
        git checkout {first_branch}
        git pull
        cargo vendor --versioned-dirs 1> .cargo/{first_branch}.config.toml
        cp .cargo/{first_branch}.config.toml .cargo/config.toml
    "}
    .play()?;
    for branch in branches.iter().skip(1) {
        my_bash! {"
            git checkout {branch}
            git pull
            cargo vendor --no-delete --versioned-dirs 1> .cargo/{branch}.config.toml
        "}
        .play()?;
    }
    my_bash! {"git checkout {first_branch}"}.play()?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum VendorSpaceError {
    #[error("{0}")]
    Config(#[from] ConfigError),

    #[error("{0}")]
    Shell(#[from] ShellError),

    #[error("{0}")]
    Repository(#[from] RepositoryError),
}

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("{0} is supposed to be used for a repository, but it is a file")]
    PathIsFile(String),

    #[error("The repository directory {0} was expected not to exist, but it already does.")]
    DirectoryAlreadyExists(String),

    #[error("The repository directory {0} is a git repository with uncommitted changes.")]
    DirtyWorkingTree(String),

    #[error("The repository directory {0} is not a valid git repository.")]
    InvalidGitRepo(String),

    #[error("Failed to clone git repo due to shell error: {0}")]
    GitClone(ShellError),
}
