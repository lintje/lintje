use std::ffi::OsStr;
use std::process::{Command, Output};

#[derive(Debug)]
pub enum ExitError {
    Exit(std::io::ErrorKind, String),
    Termination(Output), // Container systems
}

impl ExitError {
    pub fn from_error(error: &std::io::Error) -> Self {
        Self::Exit(error.kind(), error.to_string())
    }

    pub fn from_output(output: Output) -> Self {
        Self::Termination(output)
    }

    pub fn code(&self) -> Option<i32> {
        match self {
            Self::Exit(..) => None,
            Self::Termination(output) => output.status.code(),
        }
    }

    pub fn is_exit_code(&self, code: i32) -> bool {
        match self {
            Self::Exit(..) => false,
            Self::Termination(output) => output.status.code() == Some(code),
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::Exit(_kind, message) => message.trim().to_string(),
            Self::Termination(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                stderr + &stdout
            }
        }
    }
}

pub struct FailedCommand {
    pub command: String,
    pub arguments: Vec<String>,
    pub error: ExitError,
}

impl FailedCommand {
    pub fn message(&self) -> String {
        self.error.message()
    }
}

impl std::fmt::Display for FailedCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error.message())
    }
}

impl std::fmt::Debug for FailedCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Failed to run command.\n\
            Command: {}\n\
            Arguments: {:?}\n\
            Exit code: {:?}\n\
            Output:\n{}",
            self.command,
            self.arguments,
            self.error.code(),
            self.message(),
        )
    }
}

pub fn run_command<S: AsRef<OsStr> + std::fmt::Display>(
    cmd: &str,
    args: &[S],
) -> Result<String, FailedCommand> {
    let mut command = Command::new(cmd);
    command.args(args);
    match command.output() {
        Ok(output) => {
            let status = output.status;
            let stdout = String::from_utf8_lossy(&output.stdout);
            if status.success() {
                Ok(stdout.to_string())
            } else {
                // The program was run, but exited with a failure.
                //
                // Processes that fail in containers because the executable could not be found are
                // also reported this away instead of an Err.
                Err(FailedCommand {
                    command: cmd.to_string(),
                    arguments: args.iter().map(|s| s.to_string()).collect::<Vec<String>>(),
                    error: ExitError::from_output(output),
                })
            }
        }
        // Errors about scenarios like: the executable could not be found
        Err(error) => Err(FailedCommand {
            command: cmd.to_string(),
            arguments: args.iter().map(|s| s.to_string()).collect::<Vec<String>>(),
            error: ExitError::from_error(&error),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::run_command;
    use std::os::unix::process::ExitStatusExt;
    use std::process::{ExitStatus, Output};

    #[test]
    fn command_success() {
        match run_command("echo", &["-n", "123", "456"]) {
            Ok(result) => assert_eq!(result, "123 456"),
            Err(e) => panic!("Unexpected failure: {:?}", e),
        }
    }

    #[test]
    fn command_exit_failure() {
        match run_command("support/test/failure_script", &["5", "hello"]) {
            Ok(result) => panic!("Unexpected success: {:?}", result),
            Err(e) => {
                let message = "Failed to run command.\n\
                    Command: support/test/failure_script\n\
                    Arguments: [\"5\", \"hello\"]\n\
                    Exit code: Some(5)\n\
                    Output:\nSTDERR message\nSTDOUT message\n";
                assert_eq!(format!("{e:?}"), message);
                assert!(e.error.is_exit_code(5));
            }
        }
    }

    #[test]
    fn command_run_does_not_exist() {
        match run_command("support/test/failure_script", &["127", "hello"]) {
            Ok(result) => panic!("Unexpected success: {:?}", result),
            Err(e) => {
                let message = "Failed to run command.\n\
                    Command: support/test/failure_script\n\
                    Arguments: [\"127\", \"hello\"]\n\
                    Exit code: Some(127)\n\
                    Output:\nSTDERR message\nSTDOUT message\n";
                assert_eq!(format!("{e:?}"), message);
                assert!(e.error.is_exit_code(127));
            }
        }
    }

    #[test]
    fn command_run_failure() {
        match run_command("lintje-does-not-exist", &["123", "hello"]) {
            Ok(result) => panic!("Unexpected success: {:?}", result),
            Err(e) => {
                let message = "Failed to run command.\n\
                    Command: lintje-does-not-exist\n\
                    Arguments: [\"123\", \"hello\"]\n\
                    Exit code: None\n\
                    Output:\nNo such file or directory (os error 2)";
                assert_eq!(format!("{e:?}"), message);
                // No exit code because the executable could not be found
                assert!(!e.error.is_exit_code(0));
                assert!(!e.error.is_exit_code(1));
                assert!(!e.error.is_exit_code(2));
                assert!(!e.error.is_exit_code(123));
                assert!(!e.error.is_exit_code(127));
            }
        }
    }

    #[test]
    fn exit_error_message() {
        let output = Output {
            status: ExitStatus::from_raw(1),
            stdout: "STDOUT message\n".as_bytes().to_vec(),
            stderr: "STDERR message\n".as_bytes().to_vec(),
        };
        let error = super::ExitError::from_output(output);
        let message = "STDERR message\nSTDOUT message\n";
        assert_eq!(error.message(), message);
    }
}
