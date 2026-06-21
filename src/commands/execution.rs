use std::fs::{File, OpenOptions};
use std::io::Write;

use crate::commands::external::{
    find_command_in_path, run_external_command, run_external_command_background,
};
use crate::parser::ast::{ASTNode, RedirectStream};
use crate::registry::command_registry::CommandRegistry;
use crate::shell::shell_context::{BackgroundJobStatus, ShellContext};

pub struct CommandExecution<'a> {
    pub command: &'a ASTNode,
    pub stdout_redirect: Option<(String, bool)>,
    pub stderr_redirect: Option<(String, bool)>,
    pub background: bool,
}

pub fn execute_ast(
    node: &ASTNode,
    registry: &CommandRegistry,
    context: &mut ShellContext,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<(), String> {
    if matches!(node, ASTNode::Pipe { .. }) {
        return execute_pipe(node, registry, context, stdout, stderr);
    }

    let execution = flatten_command_execution(node)?;
    let mut redirected_stdout = match execution.stdout_redirect.as_ref() {
        Some((path, append)) => Some(open_redirect_file(path, *append)?),
        None => None,
    };
    let mut redirected_stderr = match execution.stderr_redirect.as_ref() {
        Some((path, append)) => Some(open_redirect_file(path, *append)?),
        None => None,
    };

    let command_result = match execution.command {
        ASTNode::Command { name, args } => {
            if name == "type" || registry.get_builtin(name).is_some() {
                if execution.background {
                    return Err("background builtins are not supported yet".to_string());
                }

                let stdout_writer = redirected_stdout
                    .as_mut()
                    .map(|file| file as &mut dyn Write)
                    .unwrap_or(stdout);
                execute_command(name, args, registry, context, stdout_writer, None, None)
            } else if execution.background {
                let command_string = build_command_string(name, args);
                run_external_command_background(
                    name,
                    args,
                    find_command_in_path(name).ok_or_else(|| format!("{name}: not found"))?,
                    stdout,
                    redirected_stdout.take(),
                    redirected_stderr.take(),
                    context,
                    command_string,
                )
            } else {
                execute_command(
                    name,
                    args,
                    registry,
                    context,
                    stdout,
                    redirected_stdout.take(),
                    redirected_stderr.take(),
                )
            }
        }
        ASTNode::Background { command } => {
            let execution = flatten_command_execution(command)?;
            if let ASTNode::Command { name, args } = execution.command {
                if name == "type" || registry.get_builtin(name).is_some() {
                    return Err("background builtins are not supported yet".to_string());
                }
                let command_string = build_command_string(name, args);
                run_external_command_background(
                    name,
                    args,
                    find_command_in_path(name).ok_or_else(|| format!("{name}: not found"))?,
                    stdout,
                    redirected_stdout.take(),
                    redirected_stderr.take(),
                    context,
                    command_string,
                )
            } else {
                Err("unsupported background command".to_string())
            }
        }
        _ => Err("unsupported command".to_string()),
    };

    match command_result {
        Err(error) if execution.stderr_redirect.is_some() => {
            if !error.is_empty() {
                let (path, append) = execution.stderr_redirect.as_ref().unwrap();
                let mut error_file =
                    open_redirect_file(path, *append).map_err(|write_error| write_error.to_string())?;
                writeln!(error_file, "{error}").map_err(|write_error| write_error.to_string())?;
            }
            Err(String::new())
        }
        result => result,
    }
}

pub fn flatten_command_execution(node: &ASTNode) -> Result<CommandExecution<'_>, String> {
    let mut current = node;
    let mut stdout_redirect = None;
    let mut stderr_redirect = None;
    let mut background = false;

    loop {
        match current {
            ASTNode::Redirect {
                command,
                file,
                stream,
            } => {
                match stream {
                    RedirectStream::Stdout => stdout_redirect = Some((file.clone(), false)),
                    RedirectStream::StdoutAppend => stdout_redirect = Some((file.clone(), true)),
                    RedirectStream::Stderr => stderr_redirect = Some((file.clone(), false)),
                    RedirectStream::StderrAppend => stderr_redirect = Some((file.clone(), true)),
                }
                current = command;
            }
            ASTNode::Background { command } => {
                background = true;
                current = command;
            }
            ASTNode::Command { .. } => {
                return Ok(CommandExecution {
                    command: current,
                    stdout_redirect,
                    stderr_redirect,
                    background,
                });
            }
            ASTNode::Pipe { .. } => return Err("pipes are parsed but not executed yet".to_string()),
        }
    }
}

pub fn reap_and_print_done_jobs(
    context: &mut ShellContext,
    stdout: &mut dyn Write,
) -> Result<(), String> {
    let statuses = context.collect_job_statuses();
    for (index, status) in statuses.iter().enumerate() {
        if let BackgroundJobStatus::Done(job_id, command) = status {
            let marker = if index + 1 == statuses.len() {
                '+'
            } else if index + 1 == statuses.len() - 1 {
                '-'
            } else {
                ' '
            };
            writeln!(stdout, "[{job_id}]{marker}  {:24} {command}", "Done")
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn execute_pipe(
    node: &ASTNode,
    registry: &CommandRegistry,
    context: &mut ShellContext,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<(), String> {
    #[cfg(unix)]
    {
        let commands = extract_pipeline_commands(node);
        if commands.is_empty() {
            return Err("empty pipeline".to_string());
        }

        if commands.len() == 1 {
            return execute_ast(&commands[0], registry, context, stdout, stderr);
        }

        return execute_pipeline_stages(&commands, registry, context);
    }

    #[cfg(not(unix))]
    {
        let _ = (node, registry, context, stdout, stderr);
        Err("pipelines are only supported on Unix".to_string())
    }
}

#[cfg(unix)]
fn extract_pipeline_commands(node: &ASTNode) -> Vec<ASTNode> {
    match node {
        ASTNode::Pipe { left, right } => {
            let mut commands = extract_pipeline_commands(left);
            commands.push((**right).clone());
            commands
        }
        other => vec![other.clone()],
    }
}

#[cfg(unix)]
fn execute_pipeline_stages(
    commands: &[ASTNode],
    registry: &CommandRegistry,
    context: &mut ShellContext,
) -> Result<(), String> {
    if commands.len() == 2 {
        execute_two_stage_pipeline(&commands[0], &commands[1], registry, context)
    } else {
        execute_multi_stage_pipeline(commands, registry, context)
    }
}

#[cfg(unix)]
fn execute_two_stage_pipeline(
    first: &ASTNode,
    second: &ASTNode,
    registry: &CommandRegistry,
    context: &mut ShellContext,
) -> Result<(), String> {
    let first_exec = flatten_command_execution(first)?;
    let second_exec = flatten_command_execution(second)?;

    let (first_name, first_args, first_is_builtin) =
        extract_command_info(first_exec.command, registry)?;
    let (second_name, second_args, second_is_builtin) =
        extract_command_info(second_exec.command, registry)?;

    #[cfg(unix)]
    {
        use std::io::Read;
        use std::process::{Command, Stdio};
        use std::os::unix::io::FromRawFd;
        use crate::commands::external::{
            execute_external_command_with_stdin, spawn_external_command,
        };

        let mut fds: [libc::c_int; 2] = [0; 2];
        if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
            return Err("failed to create pipe".to_string());
        }
        let pipe_read_fd = fds[0];
        let pipe_write_fd = fds[1];

        let pipe_read = unsafe { File::from_raw_fd(pipe_read_fd) };
        let pipe_write = unsafe { File::from_raw_fd(pipe_write_fd) };

        if first_is_builtin {
            execute_builtin_with_output(&first_name, &first_args, registry, context, pipe_write)?;

            if second_is_builtin {
                let mut stdin_buffer = Vec::new();
                let mut file = pipe_read;
                file.read_to_end(&mut stdin_buffer)
                    .map_err(|e| format!("failed to read pipe: {e}"))?;

                execute_builtin_with_input(
                    &second_name,
                    &second_args,
                    registry,
                    context,
                    &stdin_buffer,
                )?;
            } else {
                execute_external_command_with_stdin(&second_name, &second_args, pipe_read)?;
            }
        } else if second_is_builtin {
            let mut child = spawn_external_command(&first_name, &first_args, pipe_write)?;

            let mut stdin_buffer = Vec::new();
            let mut file = pipe_read;
            file.read_to_end(&mut stdin_buffer)
                .map_err(|e| format!("failed to read pipe: {e}"))?;

            child
                .wait()
                .map_err(|e| format!("failed to wait for command: {e}"))?;

            execute_builtin_with_input(
                &second_name,
                &second_args,
                registry,
                context,
                &stdin_buffer,
            )?;
        } else {
            let mut first_child = spawn_external_command(&first_name, &first_args, pipe_write)?;

            let mut second_child = Command::new(
                find_command_in_path(&second_name)
                    .ok_or_else(|| format!("{second_name}: not found"))?,
            );
            second_child.args(&second_args).stdin(Stdio::from(pipe_read));

            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;

                second_child.arg0(&second_name);
            }

            let mut second_child = second_child
                .spawn()
                .map_err(|e| format!("failed to spawn {second_name}: {e}"))?;

            first_child
                .wait()
                .map_err(|e| format!("failed to wait for {first_name}: {e}"))?;
            second_child
                .wait()
                .map_err(|e| format!("failed to wait for {second_name}: {e}"))?;
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = (first_name, first_args, first_is_builtin, second_name, second_args, second_is_builtin);
        Err("pipelines are only supported on Unix".to_string())
    }
}

#[cfg(unix)]
fn execute_builtin_with_output(
    name: &str,
    args: &[String],
    registry: &CommandRegistry,
    context: &mut ShellContext,
    stdout_pipe: File,
) -> Result<(), String> {
    if let Some(command) = registry.get_builtin(name) {
        let mut file = stdout_pipe;
        command.execute(args.to_vec(), context, &mut file)?;
        file.flush().map_err(|e| e.to_string())?;
        Ok(())
    } else if name == "type" {
        let mut file = stdout_pipe;
        run_type_command(args, registry, &mut file)?;
        file.flush().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err(format!("{name}: not found"))
    }
}

#[cfg(unix)]
fn execute_builtin_with_input(
    name: &str,
    args: &[String],
    registry: &CommandRegistry,
    context: &mut ShellContext,
    _stdin_buffer: &[u8],
) -> Result<(), String> {
    let mut stdout = std::io::stdout().lock();

    if let Some(command) = registry.get_builtin(name) {
        command.execute(args.to_vec(), context, &mut stdout)?;
    } else if name == "type" {
        run_type_command(args, registry, &mut stdout)?;
    } else {
        return Err(format!("{name}: not found"));
    }

    Ok(())
}

#[cfg(unix)]
fn extract_command_info(
    node: &ASTNode,
    registry: &CommandRegistry,
) -> Result<(String, Vec<String>, bool), String> {
    match node {
        ASTNode::Command { name, args } => {
            let is_builtin = name == "type" || registry.get_builtin(name).is_some();
            Ok((name.clone(), args.clone(), is_builtin))
        }
        _ => Err("unsupported command in pipeline".to_string()),
    }
}

#[cfg(unix)]
fn execute_multi_stage_pipeline(
    commands: &[ASTNode],
    registry: &CommandRegistry,
    context: &mut ShellContext,
) -> Result<(), String> {
    if commands.is_empty() {
        return Err("empty pipeline".to_string());
    }

    if commands.len() <= 2 {
        return execute_pipeline_stages(commands, registry, context);
    }

    #[cfg(unix)]
    {
        use std::os::unix::io::FromRawFd;

        let num_commands = commands.len();
        let num_pipes = num_commands - 1;
        let mut pipes: Vec<(i32, i32)> = Vec::with_capacity(num_pipes);

        for _ in 0..num_pipes {
            let mut fds: [libc::c_int; 2] = [0; 2];
            if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
                return Err("failed to create pipe".to_string());
            }
            pipes.push((fds[0], fds[1]));
        }

        let mut children = Vec::new();

        for (i, cmd) in commands.iter().enumerate() {
            let execution = flatten_command_execution(cmd)?;
            let (cmd_name, cmd_args, _is_builtin) =
                extract_command_info(execution.command, registry)?;

            if cmd_name == "type" || registry.get_builtin(&cmd_name).is_some() {
                return Err("built-in commands in multi-stage pipelines not yet supported".to_string());
            }

            let path =
                find_command_in_path(&cmd_name).ok_or_else(|| format!("{cmd_name}: not found"))?;

            let mut cmd_obj = Command::new(path);
            cmd_obj.args(&cmd_args);

            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;

                cmd_obj.arg0(&cmd_name);
            }

            if i > 0 {
                let (read_fd, _) = pipes[i - 1];
                let stdin_file = unsafe { File::from_raw_fd(read_fd) };
                cmd_obj.stdin(Stdio::from(stdin_file));
            }

            if i < num_commands - 1 {
                let (_, write_fd) = pipes[i];
                let stdout_file = unsafe { File::from_raw_fd(write_fd) };
                cmd_obj.stdout(Stdio::from(stdout_file));
            }

            if let Some((path, append)) = execution.stdout_redirect {
                let file = open_redirect_file(&path, append)?;
                cmd_obj.stdout(Stdio::from(file));
            }

            if let Some((path, append)) = execution.stderr_redirect {
                let file = open_redirect_file(&path, append)?;
                cmd_obj.stderr(Stdio::from(file));
            }

            let child = cmd_obj
                .spawn()
                .map_err(|e| format!("failed to spawn {cmd_name}: {e}"))?;

            children.push(child);
        }

        drop(pipes);

        for mut child in children {
            child
                .wait()
                .map_err(|e| format!("failed to wait for child process: {e}"))?;
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = (commands, registry, context);
        Err("multi-stage pipelines are only supported on Unix".to_string())
    }
}

fn execute_command(
    name: &str,
    args: &[String],
    registry: &CommandRegistry,
    context: &mut ShellContext,
    stdout: &mut dyn Write,
    stdout_file: Option<File>,
    stderr_file: Option<File>,
) -> Result<(), String> {
    if name == "type" {
        return run_type_command(args, registry, stdout);
    }

    if let Some(command) = registry.get_builtin(name) {
        return command.execute(args.to_vec(), context, stdout);
    }

    let executable_path = find_command_in_path(name).ok_or_else(|| format!("{name}: not found"))?;
    run_external_command(name, args, executable_path, stdout_file, stderr_file)
}

fn run_type_command(
    args: &[String],
    registry: &CommandRegistry,
    stdout: &mut dyn Write,
) -> Result<(), String> {
    let target = args
        .first()
        .ok_or_else(|| "type: missing argument".to_string())?;

    if registry.get_builtin(target).is_some() || target == "type" {
        writeln!(stdout, "{target} is a shell builtin").map_err(|error| error.to_string())?;
    } else if let Some(path) = find_command_in_path(target) {
        writeln!(stdout, "{target} is {}", path.display()).map_err(|error| error.to_string())?;
    } else {
        writeln!(stdout, "{target}: not found").map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn build_command_string(name: &str, args: &[String]) -> String {
    if args.is_empty() {
        name.to_string()
    } else {
        let mut pieces = Vec::with_capacity(args.len() + 1);
        pieces.push(name.to_string());
        pieces.extend(args.iter().cloned());
        pieces.join(" ")
    }
}

fn open_redirect_file(path: &str, append: bool) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.write(true).create(true);

    if append {
        options.append(true);
    } else {
        options.truncate(true);
    }

    options.open(path).map_err(|error| error.to_string())
}
