use std::ffi::OsStr;
use std::process::Command;

pub struct CommandError {
    pub code: Option<i32>,
    pub message: String,
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::fmt::Debug for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

pub fn run_command<S: AsRef<OsStr> + std::fmt::Debug>(
    cmd: &str,
    args: &[S],
) -> Result<String, CommandError> {
    let mut command = Command::new(cmd);
    command.args(args);
    match command.output() {
        Ok(output) => {
            let status = output.status;
            let stdout = String::from_utf8_lossy(&output.stdout);
            if status.success() {
                Ok(stdout.to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let status_code = status.code();
                let (exit_code, additional_message) = match status_code {
                    Some(127) => {
                        // I've only seen this happen on emulated systems: host
                        // architecture is different from the Docker image.
                        // Otherwise it returns the OS error ErrorKind::NotFound.
                        ("127".to_string(), " Is it installed?")
                    }
                    Some(code) => (code.to_string(), ""),
                    None => ("unknown".to_string(), ""),
                };
                return Err(CommandError {
                    code: status_code,
                    message: format!(
                        "Failed to run command.{}\n\
                        Command: {}\n\
                        Arguments: {:?}\n\
                        Exit code: {}\n\
                        STDOUT: {}\n\
                        STDERR: {}",
                        additional_message, cmd, args, exit_code, stdout, stderr
                    ),
                });
            }
        }
        Err(e) => {
            let additional_message = if e.kind() == std::io::ErrorKind::NotFound {
                " Is it installed?"
            } else {
                ""
            };
            return Err(CommandError {
                code: None,
                message: format!(
                    "Failed to run command.{}\n\
                    Command: {}\n\
                    Arguments: {:?}\n\
                    Error: {}",
                    additional_message, cmd, args, e
                ),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run_command;

    #[test]
    fn test_command_success() {
        match run_command("echo", &vec!["-n", "123", "456"]) {
            Ok(result) => assert_eq!(result, "123 456"),
            Err(e) => panic!("Unexpected failure: {:?}", e),
        }
    }

    #[test]
    fn test_command_exit_failure() {
        match run_command("support/test/failure_script", &vec!["5", "hello"]) {
            Ok(result) => panic!("Unexpected success: {:?}", result),
            Err(e) => {
                let message = "Failed to run command.\n\
                    Command: support/test/failure_script\n\
                    Arguments: [\"5\", \"hello\"]\n\
                    Exit code: 5\n\
                    STDOUT: STDOUT message\n\n\
                    STDERR: STDERR message\n";
                assert_eq!(e.message, message)
            }
        }
    }

    #[test]
    fn test_command_run_does_not_exist() {
        match run_command("support/test/failure_script", &vec!["127", "hello"]) {
            Ok(result) => panic!("Unexpected success: {:?}", result),
            Err(e) => {
                let message = "Failed to run command. Is it installed?\n\
                    Command: support/test/failure_script\n\
                    Arguments: [\"127\", \"hello\"]\n\
                    Exit code: 127\n\
                    STDOUT: STDOUT message\n\n\
                    STDERR: STDERR message\n";
                assert_eq!(e.message, message)
            }
        }
    }

    #[test]
    fn test_command_run_failure() {
        match run_command("lintje-does-not-exist", &vec!["123", "hello"]) {
            Ok(result) => panic!("Unexpected success: {:?}", result),
            Err(e) => {
                let message = "Failed to run command. Is it installed?\n\
                    Command: lintje-does-not-exist\n\
                    Arguments: [\"123\", \"hello\"]\n\
                    Error: No such file or directory (os error 2)";
                assert_eq!(e.message, message)
            }
        }
    }
}
