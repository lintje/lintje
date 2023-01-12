use std::fs::OpenOptions;
use std::io::Write;
#[cfg(target_family = "unix")]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[derive(clap::ArgEnum, Clone, Debug)]
pub enum CommitHook {
    CommitMsg,
    PostCommit,
}

impl CommitHook {
    pub fn name(&self) -> &str {
        match self {
            CommitHook::CommitMsg => "commit-msg",
            CommitHook::PostCommit => "post-commit",
        }
    }

    pub fn command(&self) -> &str {
        match self {
            CommitHook::CommitMsg => "\n\nlintje --hook-message-file=$1",
            CommitHook::PostCommit => "\n\nlintje",
        }
    }
}

pub fn install_hook(hook: &CommitHook) -> Result<String, String> {
    let file_path = format!(".git/hooks/{}", hook.name());
    let hook_file = Path::new(&file_path);
    match OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(hook_file)
    {
        Ok(mut file) => {
            if let Err(e) = set_file_permissions(hook_file) {
                return Err(format!(
                    "Cannot set file permissions for: {:?}\n{:?}",
                    hook_file, e
                ));
            }
            let hook_content = hook.command();
            match file.write_all(hook_content.as_bytes()) {
                Ok(()) => Ok(hook_file.to_str().unwrap().to_string()),
                Err(e) => Err(format!(
                    "Unable to open write to Git hook file: {}\n{}",
                    hook_file.to_str().unwrap(),
                    e
                )),
            }
        }
        Err(e) => Err(format!(
            "Unable to open Git hook file: {}\n{}",
            hook_file.to_str().unwrap(),
            e
        )),
    }
}

#[cfg(target_family = "unix")]
fn set_file_permissions(path: &Path) -> std::io::Result<()> {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o744))
}

#[cfg(not(target_family = "unix"))]
fn set_file_permissions(file: &Path) -> std::io::Result<()> {
    Ok(())
}
