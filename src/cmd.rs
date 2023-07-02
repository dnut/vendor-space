use std::{io, process::Command};
use thiserror::Error;

crate::make_bash!(pub bash);
crate::make_bash!(pub safe_bash: "set -euxo pipefail");

#[macro_export]
macro_rules! make_bash {
    (pub $name:ident $(: $($prefix:tt)*)?) => {
        $crate::make_bash!(($) #[macro_export] $name $(: $($prefix)*)?);
    };
    ($viz:vis $name:ident $(: $($prefix:tt)*)?) => {
        make_bash!(($) $name $(: $($prefix)*)?);
        $viz use $name;
    };
    (($dollar:tt) $(#[$outer:meta])* $name:ident $(: $($prefix:tt)*)?) => {
        $(#[$outer])*
        macro_rules! $name {
            ($dollar ($dollar arg:tt)*) => {{
                use std::process::Command;
                use $crate::cmd::CommandExt;
                #[allow(unused_mut, unused_assignments)]
                let mut prefix = "".to_string();
                $(prefix = format!($($prefix)*);)?
                Command::new("bash").arg("-c").arg(
                    &format!(
                        "{}\n{}",
                        &prefix,
                        &format!($dollar($arg)*)
                    )
                ).play()
            }};
        }
    };
}

pub trait CommandExt {
    /// execute the command, using the stdio of the parent process.
    /// return after command completes.
    /// Ok(()) indicates completion with 0 exit code
    fn play(&mut self) -> Result<(), ShellError>;
}
impl CommandExt for Command {
    fn play(&mut self) -> Result<(), ShellError> {
        let status = self.status()?;
        if !status.success() {
            return Err(ShellError::BadExitCode(status.code()));
        }
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum ShellError {
    #[error("failed to run command due to IO Error: {0}")]
    IoError(#[from] io::Error),

    #[error("Command exited with unexpected exit code {0:?}")]
    BadExitCode(Option<i32>),
}
