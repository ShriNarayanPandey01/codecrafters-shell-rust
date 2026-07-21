use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::shell::shell_context::ShellContext;

pub struct ExternalCommandIo<'a> {
    pub stdout: &'a mut dyn Write,
    pub stderr: &'a mut dyn Write,
    pub stdout_file: Option<File>,
    pub stderr_file: Option<File>,
}

pub struct BackgroundCommandRequest<'a> {
    pub name: &'a str,
    pub args: &'a [String],
    pub executable_path: PathBuf,
    pub stdout: &'a mut dyn Write,
    pub stdout_file: Option<File>,
    pub stderr_file: Option<File>,
    pub context: &'a mut ShellContext,
    pub command_string: String,
}

pub fn build_command_for_path(
    executable_path: PathBuf,
    args: &[String],
) -> Result<Command, String> {
    build_external_command(args, executable_path)
}

pub fn find_command_in_path(command: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;

    let command_path = Path::new(command);
    let has_extension = command_path.extension().is_some();

    fn resolve_search_dir(directory: PathBuf) -> PathBuf {
        if directory.is_absolute() {
            directory
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(directory)
        }
    }

    let executable = std::env::split_paths(&path_var).find_map(|directory| {
        let directory = resolve_search_dir(directory);
        if has_extension {
            let candidate = directory.join(command);
            if is_executable_file(&candidate) {
                return Some(candidate);
            }
        } else {
            let candidate = directory.join(command);
            if is_executable_file(&candidate) {
                return Some(candidate);
            }

            #[cfg(windows)]
            if let Some(path) = find_in_pathext(&directory, command) {
                return Some(path);
            }
        }

        None
    });

    if executable.is_some() {
        return executable;
    }

    if has_extension {
        None
    } else {
        // Fall back to files without extension when the command is typed without one.
        std::env::split_paths(&path_var).find_map(|directory| {
            let candidate = directory.join(command);
            if is_executable_file(&candidate) {
                Some(candidate)
            } else {
                None
            }
        })
    }
}

#[cfg(windows)]
fn find_in_pathext(directory: &Path, command: &str) -> Option<PathBuf> {
    let pathext = std::env::var_os("PATHEXT").unwrap_or_default();
    let extensions = std::env::split_paths(&pathext)
        .filter_map(|path| path.to_str().map(|s| s.to_ascii_lowercase()))
        .map(|s| {
            if s.starts_with('.') {
                s
            } else {
                format!(".{s}")
            }
        })
        .collect::<Vec<_>>();

    for extension in extensions {
        let candidate = directory.join(format!("{command}{extension}"));
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
    }

    None
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

#[cfg(windows)]
fn resolve_relative_windows_path(executable_path: PathBuf, current_dir: &Path) -> PathBuf {
    if executable_path.is_absolute() {
        executable_path
    } else {
        current_dir.join(executable_path)
    }
}

pub fn run_external_command(
    name: &str,
    args: &[String],
    executable_path: PathBuf,
    current_dir: &Path,
    io: ExternalCommandIo<'_>,
) -> Result<(), String> {
    #[cfg(windows)]
    let executable_path = resolve_relative_windows_path(executable_path, current_dir);
    let mut command = build_command_for_path(executable_path, args)?;
    command.current_dir(current_dir);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        command.arg0(name);
    }

    if let Some(file) = io.stdout_file {
        command.stdout(Stdio::from(file));
    } else {
        command.stdout(Stdio::piped());
    }

    if let Some(file) = io.stderr_file {
        command.stderr(Stdio::from(file));
    } else {
        command.stderr(Stdio::piped());
    }

    let output = command
        .output()
        .map_err(|error| format!("failed to execute {name}: {error}"))?;

    io.stdout
        .write_all(&output.stdout)
        .map_err(|error| error.to_string())?;
    io.stderr
        .write_all(&output.stderr)
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(windows)]
fn find_windows_shell_interpreter() -> Option<PathBuf> {
    find_command_in_path("sh").or_else(|| find_command_in_path("bash"))
}

#[cfg(windows)]
fn escape_shell_single_quoted(value: &str) -> String {
    if !value.contains('"') && !value.contains('`') && !value.contains('$') {
        return format!("'{}'", value.replace('\'', "'\\''"));
    }

    let mut escaped = String::new();
    escaped.push('\'');
    for ch in value.chars() {
        if ch == '\'' {
            escaped.push_str("'\\''");
        } else {
            escaped.push(ch);
        }
    }
    escaped.push('\'');
    escaped
}

#[cfg(windows)]
fn windows_path_to_bash_path(path: &Path) -> Option<String> {
    let path_str = path.to_str()?;
    let mut chars = path_str.chars();
    let drive = chars.next()?;
    let colon = chars.next()?;
    if !drive.is_ascii_alphabetic() || colon != ':' {
        return None;
    }

    let rest: String = chars.map(|c| if c == '\\' { '/' } else { c }).collect();
    let drive_letter = drive.to_ascii_lowercase();
    Some(format!("/mnt/{drive_letter}{rest}"))
}

#[cfg(windows)]
fn build_windows_shell_command(
    interpreter: &Path,
    executable_path: &Path,
    args: &[String],
) -> Result<Command, String> {
    let mut command = Command::new(interpreter);
    let interpreter_name = interpreter
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if interpreter_name == "bash.exe" || interpreter_name == "bash" {
        let bash_path = windows_path_to_bash_path(executable_path).ok_or_else(|| {
            format!(
                "failed to execute {}: unsupported Windows path",
                executable_path.display()
            )
        })?;

        let quoted_path = escape_shell_single_quoted(&bash_path);
        let wrapper = format!("exec {quoted_path} \"$@\"");
        command.arg("-lc").arg(wrapper).arg("--");
        if !args.is_empty() {
            command.args(args);
        }
    } else {
        command.arg(executable_path);
        command.args(args);
    }

    Ok(command)
}

fn build_external_command(args: &[String], executable_path: PathBuf) -> Result<Command, String> {
    #[cfg(windows)]
    {
        let extension = executable_path
            .extension()
            .and_then(|os| os.to_str())
            .map(|s| s.to_ascii_lowercase());

        if matches!(extension.as_deref(), Some("sh")) {
            let interpreter = find_windows_shell_interpreter().ok_or_else(|| {
                format!(
                    "failed to execute {}: shell interpreter (sh/bash) not found in PATH",
                    executable_path.display()
                )
            })?;

            return build_windows_shell_command(&interpreter, &executable_path, args);
        }
    }

    let mut command = Command::new(executable_path);
    command.args(args);
    Ok(command)
}

#[cfg(unix)]
pub fn spawn_external_command(
    name: &str,
    args: &[String],
    current_dir: &Path,
    stdout_pipe: File,
) -> Result<Child, String> {
    let path = find_command_in_path(name).ok_or_else(|| format!("{name}: not found"))?;
    let mut cmd = Command::new(path);
    cmd.args(args).current_dir(current_dir);

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
    current_dir: &Path,
    stdin_file: File,
    stdout: &mut dyn Write,
) -> Result<(), String> {
    let path = find_command_in_path(name).ok_or_else(|| format!("{name}: not found"))?;
    let mut cmd = Command::new(path);
    cmd.args(args).current_dir(current_dir);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        cmd.arg0(name);
    }

    let output = cmd
        .stdin(Stdio::from(stdin_file))
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| format!("failed to spawn {name}: {e}"))?;

    stdout
        .write_all(&output.stdout)
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn run_external_command_background(
    request: BackgroundCommandRequest<'_>,
) -> Result<(), String> {
    #[cfg(windows)]
    let executable_path =
        resolve_relative_windows_path(request.executable_path, &request.context.current_dir_path());
    #[cfg(not(windows))]
    let executable_path = request.executable_path;

    let mut command = build_command_for_path(executable_path, request.args)?;
    command.current_dir(request.context.current_dir_path());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        command.arg0(request.name);
    }

    if let Some(file) = request.stdout_file {
        command.stdout(Stdio::from(file));
    }

    if let Some(file) = request.stderr_file {
        command.stderr(Stdio::from(file));
    }

    let child = command
        .spawn()
        .map_err(|error| format!("failed to execute {}: {error}", request.name))?;

    let job_id = request
        .context
        .add_background_job(child, request.command_string);
    let process_id = request
        .context
        .background_jobs
        .last()
        .map(|job| job.child.id())
        .ok_or_else(|| "failed to track background job".to_string())?;
    writeln!(request.stdout, "[{job_id}] {process_id}").map_err(|error| error.to_string())?;
    Ok(())
}
#[cfg(unix)]
use std::process::Child;
