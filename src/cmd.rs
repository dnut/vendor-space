use std::{io, process::Command};
use thiserror::Error;

crate::make_bash!(
    /// Prepare a command to run a string as a shell script in bash
    pub bash;

    /// Prepare a command to run a string as a shell script in bash, and:
    /// - fail the script on the first error.
    /// - fail the script if it attempts to use unset variables
    /// - print every command as it is executed.
    pub safe_bash: "set -euxo pipefail";
);

#[macro_export]
macro_rules! make_bash {
    ($(
        $(#[$attr:meta])*
        pub $name:ident $(: $($prefix:expr)*)?
    );+$(;)?) => {
        $($crate::make_bash!(($) $(#[$attr])* #[macro_export] $name $(: $($prefix)*)?);)+
    };
    ($(
        $(#[$attr:meta])*
        $viz:vis $name:ident $(: $($prefix:expr)*)?
    );+$(;)?) => {
        $(
            make_bash!(($) $(#[$attr])* $name $(: $($prefix)*)?);
            $viz use $name;
        )+
    };
    (($dollar:tt) $(#[$attr:meta])* $name:ident $(: $($prefix:expr)*)?) => {
        $(#[$attr])*
        macro_rules! $name {
            ($dollar ($dollar arg:tt)*) => {{
                use std::process::Command;
                #[allow(unused_mut, unused_assignments)]
                let mut prefix = "".to_string();
                $(prefix = format!($($prefix)*);)?
                Command::new("bash").arg("-c").arg(
                    &format!(
                        "{}\n{}",
                        &prefix,
                        &format!($dollar($arg)*)
                    )
                )
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
