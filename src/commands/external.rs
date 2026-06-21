use std::fs::File;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::shell::shell_context::ShellContext;

pub fn find_command_in_path(command: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;

    std::env::split_paths(&path_var).find_map(|directory| {
        let candidate = directory.join(command);
        if is_executable_file(&candidate) {
            Some(candidate)
        } else {
            None
        }
    })
}

fn is_executable_file(path: &PathBuf) -> bool {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return false,
    };

    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        true
    }
}

pub fn run_external_command(
    name: &str,
    args: &[String],
    executable_path: PathBuf,
    stdout_file: Option<File>,
    stderr_file: Option<File>,
) -> Result<(), String> {
    let mut command = Command::new(executable_path);
    command.args(args);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        command.arg0(name);
    }

    if let Some(file) = stdout_file {
        command.stdout(Stdio::from(file));
    }

    if let Some(file) = stderr_file {
        command.stderr(Stdio::from(file));
    }

    command
        .status()
        .map(|_| ())
        .map_err(|error| format!("failed to execute {name}: {error}"))
}

#[cfg(unix)]
pub fn spawn_external_command(
    name: &str,
    args: &[String],
    stdout_pipe: File,
) -> Result<Child, String> {
    let path = find_command_in_path(name).ok_or_else(|| format!("{name}: not found"))?;
    let mut cmd = Command::new(path);
    cmd.args(args);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        cmd.arg0(name);
    }

    cmd.stdout(Stdio::from(stdout_pipe))
        .spawn()
        .map_err(|e| format!("failed to spawn {name}: {e}"))
}

#[cfg(unix)]
pub fn execute_external_command_with_stdin(
    name: &str,
    args: &[String],
    stdin_file: File,
) -> Result<(), String> {
    let path = find_command_in_path(name).ok_or_else(|| format!("{name}: not found"))?;
    let mut cmd = Command::new(path);
    cmd.args(args);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        cmd.arg0(name);
    }

    let mut child = cmd
        .stdin(Stdio::from(stdin_file))
        .spawn()
        .map_err(|e| format!("failed to spawn {name}: {e}"))?;

    child
        .wait()
        .map_err(|e| format!("failed to wait for {name}: {e}"))?;

    Ok(())
}

pub fn run_external_command_background(
    name: &str,
    args: &[String],
    executable_path: PathBuf,
    stdout: &mut dyn std::io::Write,
    stdout_file: Option<File>,
    stderr_file: Option<File>,
    context: &mut ShellContext,
    command_string: String,
) -> Result<(), String> {
    let mut command = Command::new(executable_path);
    command.args(args);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        command.arg0(name);
    }

    if let Some(file) = stdout_file {
        command.stdout(Stdio::from(file));
    }

    if let Some(file) = stderr_file {
        command.stderr(Stdio::from(file));
    }

    let child = command
        .spawn()
        .map_err(|error| format!("failed to execute {name}: {error}"))?;

    let job_id = context.add_background_job(child, command_string);
    writeln!(stdout, "[{job_id}] {}", context.background_jobs.last().unwrap().child.id())
        .map_err(|error| error.to_string())?;
    Ok(())
}
#[cfg(unix)]
use std::process::Child;
