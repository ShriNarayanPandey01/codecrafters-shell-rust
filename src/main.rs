mod commands {
    pub mod cd;
    pub mod complete;
    pub mod declare;
    pub mod echo;
    pub mod exit;
    pub mod history;
    pub mod jobs;
    pub mod pwd;
}

mod lexers {
    pub mod lexer;
    pub mod token;
}

mod parser {
    pub mod ast;
    pub mod parser;
}

mod registry {
    pub mod command_registry;
}

mod shell {
    pub mod autocomplete;
    pub mod built_in_command;
    pub mod completion_registry;
    pub mod shell_context;
}

use std::fs::File;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio, Child};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::os::unix::process::CommandExt;

use lexers::lexer::Lexer;
use parser::ast::{ASTNode, RedirectStream};
use parser::parser::Parser;
use registry::command_registry::CommandRegistry;
use rustyline::Editor;
use rustyline::config::{BellStyle, CompletionType, Config};
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use shell::autocomplete::ShellAutocomplete;
use shell::completion_registry::CompletionRegistry;
use shell::shell_context::{BackgroundJobStatus, ShellContext};

/// Execute a pipeline (connected commands with |)
fn execute_pipe(
    node: &ASTNode,
    registry: &CommandRegistry,
    context: &mut ShellContext,
    _stdout: &mut dyn Write,
    _stderr: &mut dyn Write,
) -> Result<(), String> {
    // Extract all commands in the pipeline
    let commands = extract_pipeline_commands(node);
    if commands.is_empty() {
        return Err("empty pipeline".to_string());
    }

    // For a single command (shouldn't happen, but handle it)
    if commands.len() == 1 {
        return execute_ast(&commands[0], registry, context, _stdout, _stderr);
    }

    // Handle pipelines: build the pipeline stages and execute
    execute_pipeline_stages(&commands, registry, context)
}

/// Extract commands from a pipeline AST node
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

/// Execute a multi-stage pipeline, handling both built-in and external commands
fn execute_pipeline_stages(
    commands: &[ASTNode],
    registry: &CommandRegistry,
    context: &mut ShellContext,
) -> Result<(), String> {
    if commands.len() == 2 {
        // Special handling for 2-command pipelines (most common case)
        execute_two_stage_pipeline(&commands[0], &commands[1], registry, context)
    } else {
        // For pipelines with more than 2 commands
        execute_multi_stage_pipeline(commands, registry, context)
    }
}

/// Execute a two-stage pipeline
fn execute_two_stage_pipeline(
    first: &ASTNode,
    second: &ASTNode,
    registry: &CommandRegistry,
    context: &mut ShellContext,
) -> Result<(), String> {
    let first_exec = flatten_command_execution(first)?;
    let second_exec = flatten_command_execution(second)?;

    let (first_name, first_args, first_is_builtin) = extract_command_info(&first_exec.command, registry)?;
    let (second_name, second_args, second_is_builtin) = extract_command_info(&second_exec.command, registry)?;

    // Create a pipe for communication between stages
    #[cfg(unix)]
    {
        use std::os::unix::io::FromRawFd;
        
        let mut fds: [libc::c_int; 2] = [0; 2];
        if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
            return Err("failed to create pipe".to_string());
        }
        let pipe_read_fd = fds[0];
        let pipe_write_fd = fds[1];
        
        let pipe_read = unsafe { std::fs::File::from_raw_fd(pipe_read_fd) };
        let pipe_write = unsafe { std::fs::File::from_raw_fd(pipe_write_fd) };

        if first_is_builtin {
            // Execute first built-in command with piped output
            execute_builtin_with_output(
                &first_name,
                &first_args,
                registry,
                context,
                pipe_write,
            )?;

            // Execute second command with piped input
            if second_is_builtin {
                let mut stdin_buffer = Vec::new();
                let mut file = pipe_read;
                std::io::Read::read_to_end(&mut file, &mut stdin_buffer)
                    .map_err(|e| format!("failed to read pipe: {e}"))?;

                execute_builtin_with_input(
                    &second_name,
                    &second_args,
                    registry,
                    context,
                    &stdin_buffer,
                )?;
            } else {
                execute_external_command_with_stdin(
                    &second_name,
                    &second_args,
                    pipe_read,
                )?;
            }
        } else {
            // First command is external
            if second_is_builtin {
                // Spawn first external command with piped output
                let mut child = spawn_external_command(&first_name, &first_args, pipe_write)?;

                // Read its output
                let mut stdin_buffer = Vec::new();
                let mut file = pipe_read;
                std::io::Read::read_to_end(&mut file, &mut stdin_buffer)
                    .map_err(|e| format!("failed to read pipe: {e}"))?;

                // Wait for first command
                child
                    .wait()
                    .map_err(|e| format!("failed to wait for command: {e}"))?;

                // Execute second built-in with input from first command
                execute_builtin_with_input(
                    &second_name,
                    &second_args,
                    registry,
                    context,
                    &stdin_buffer,
                )?;
            } else {
                // Both are external: use simpler process spawning
                let mut first_child = spawn_external_command(&first_name, &first_args, pipe_write)?;

                let mut second_child = Command::new(find_command_in_path(&second_name)
                    .ok_or_else(|| format!("{second_name}: not found"))?)
                    .args(&second_args)
                    .stdin(Stdio::from(pipe_read))
                    .spawn()
                    .map_err(|e| format!("failed to spawn {second_name}: {e}"))?;

                first_child
                    .wait()
                    .map_err(|e| format!("failed to wait for {first_name}: {e}"))?;
                second_child
                    .wait()
                    .map_err(|e| format!("failed to wait for {second_name}: {e}"))?;
            }
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        Err("pipelines are only supported on Unix".to_string())
    }
}

/// Spawn an external command with piped output
fn spawn_external_command(
    name: &str,
    args: &[String],
    stdout_pipe: std::fs::File,
) -> Result<Child, String> {
    let path = find_command_in_path(name).ok_or_else(|| format!("{name}: not found"))?;
    let mut cmd = Command::new(path);
    cmd.args(args);

    #[cfg(unix)]
    cmd.arg0(name);

    cmd.stdout(Stdio::from(stdout_pipe))
        .spawn()
        .map_err(|e| format!("failed to spawn {name}: {e}"))
}

/// Execute external command reading from stdin
fn execute_external_command_with_stdin(
    name: &str,
    args: &[String],
    stdin_file: std::fs::File,
) -> Result<(), String> {
    let path = find_command_in_path(name).ok_or_else(|| format!("{name}: not found"))?;
    let mut cmd = Command::new(path);
    cmd.args(args);

    #[cfg(unix)]
    cmd.arg0(name);

    let mut child = cmd
        .stdin(Stdio::from(stdin_file))
        .spawn()
        .map_err(|e| format!("failed to spawn {name}: {e}"))?;

    child
        .wait()
        .map_err(|e| format!("failed to wait for {name}: {e}"))?;

    Ok(())
}

/// Execute a built-in command with output to a pipe
fn execute_builtin_with_output(
    name: &str,
    args: &[String],
    registry: &CommandRegistry,
    context: &mut ShellContext,
    stdout_pipe: std::fs::File,
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

/// Execute a built-in command with input from a buffer
fn execute_builtin_with_input(
    name: &str,
    args: &[String],
    registry: &CommandRegistry,
    context: &mut ShellContext,
    _stdin_buffer: &[u8],
) -> Result<(), String> {
    let mut stdout = io::stdout().lock();

    if let Some(command) = registry.get_builtin(name) {
        // For built-ins, we execute them with regular stdout (they'll read from their args)
        // Some built-ins like "type" don't actually read stdin, they just process arguments
        // The stdin_buffer parameter is available for built-ins that need to read stdin
        command.execute(args.to_vec(), context, &mut stdout)?;
    } else if name == "type" {
        run_type_command(args, registry, &mut stdout)?;
    } else {
        return Err(format!("{name}: not found"));
    }

    Ok(())
}

/// Extract command name, args, and whether it's a built-in from an execution node
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

/// Execute multi-stage pipeline with more than 2 commands
fn execute_multi_stage_pipeline(
    commands: &[ASTNode],
    registry: &CommandRegistry,
    _context: &mut ShellContext,
) -> Result<(), String> {
    if commands.is_empty() {
        return Err("empty pipeline".to_string());
    }

    if commands.len() <= 2 {
        // This shouldn't happen, but handle it
        return execute_pipeline_stages(commands, registry, _context);
    }

    #[cfg(unix)]
    {
        use std::os::unix::io::FromRawFd;

        // For pipelines with more than 2 commands, we create N-1 pipes for N commands
        let num_commands = commands.len();
        let num_pipes = num_commands - 1;

        // Create all pipes first
        let mut pipes: Vec<(i32, i32)> = Vec::with_capacity(num_pipes);
        for _ in 0..num_pipes {
            let mut fds: [libc::c_int; 2] = [0; 2];
            if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
                return Err("failed to create pipe".to_string());
            }
            pipes.push((fds[0], fds[1])); // (read_fd, write_fd)
        }

        let mut children = Vec::new();

        // Spawn all commands
        for (i, cmd) in commands.iter().enumerate() {
            let execution = flatten_command_execution(cmd)?;
            let (cmd_name, cmd_args, _is_builtin) = extract_command_info(&execution.command, registry)?;

            // For multi-stage pipelines, only support external commands for now
            if cmd_name == "type" || registry.get_builtin(&cmd_name).is_some() {
                return Err("built-in commands in multi-stage pipelines not yet supported".to_string());
            }

            let path = find_command_in_path(&cmd_name)
                .ok_or_else(|| format!("{cmd_name}: not found"))?;

            let mut cmd_obj = Command::new(path);
            cmd_obj.args(&cmd_args);

            #[cfg(unix)]
            cmd_obj.arg0(&cmd_name);

            // Set stdin (except for first command)
            if i > 0 {
                let (read_fd, _) = pipes[i - 1];
                let stdin_file = unsafe { File::from_raw_fd(read_fd) };
                cmd_obj.stdin(Stdio::from(stdin_file));
            }

            // Set stdout (except for last command)
            if i < num_commands - 1 {
                let (_, write_fd) = pipes[i];
                let stdout_file = unsafe { File::from_raw_fd(write_fd) };
                cmd_obj.stdout(Stdio::from(stdout_file));
            }

            // Apply redirections if specified
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

        // Close all pipe file descriptors in the parent process
        // (they're already owned by the child processes via Command)
        drop(pipes);

        // Wait for all children to complete
        for mut child in children {
            child
                .wait()
                .map_err(|e| format!("failed to wait for child process: {e}"))?;
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        Err("multi-stage pipelines are only supported on Unix".to_string())
    }
}

fn execute_ast(
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
                execute_command(name, args, registry, context, stdout_writer, None, None, false)
            } else if execution.background {
                let command_string = build_command_string(name, args);
                run_external_command_background(
                    name,
                    args,
                    find_command_in_path(name)
                        .ok_or_else(|| format!("{name}: not found"))?,
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
                    false,
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
                    find_command_in_path(name)
                        .ok_or_else(|| format!("{name}: not found"))?,
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
                let mut error_file = open_redirect_file(path, *append)
                    .map_err(|write_error| write_error.to_string())?;
                writeln!(error_file, "{error}").map_err(|write_error| write_error.to_string())?;
            }
            Err(String::new())
        }
        result => result,
    }
}

fn execute_command(
    name: &str,
    args: &[String],
    registry: &CommandRegistry,
    _context: &mut ShellContext,
    stdout: &mut dyn Write,
    stdout_file: Option<File>,
    stderr_file: Option<File>,
    _background: bool,
) -> Result<(), String> {
    if name == "type" {
        return run_type_command(args, registry, stdout);
    }

    if let Some(command) = registry.get_builtin(name) {
        return command.execute(args.to_vec(), _context, stdout);
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

fn find_command_in_path(command: &str) -> Option<PathBuf> {
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
        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        true
    }
}

fn run_external_command(
    name: &str,
    args: &[String],
    executable_path: PathBuf,
    stdout_file: Option<File>,
    stderr_file: Option<File>,
) -> Result<(), String> {
    let mut command = Command::new(executable_path);
    command.args(args);

    #[cfg(unix)]
    command.arg0(name);

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

fn run_external_command_background(
    name: &str,
    args: &[String],
    executable_path: PathBuf,
    stdout: &mut dyn Write,
    stdout_file: Option<File>,
    stderr_file: Option<File>,
    context: &mut ShellContext,
    command_string: String,
) -> Result<(), String> {
    let mut command = Command::new(executable_path);
    command.args(args);

    #[cfg(unix)]
    command.arg0(name);

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

struct CommandExecution<'a> {
    command: &'a ASTNode,
    stdout_redirect: Option<(String, bool)>,
    stderr_redirect: Option<(String, bool)>,
    background: bool,
}

fn flatten_command_execution(node: &ASTNode) -> Result<CommandExecution<'_>, String> {
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

fn reap_and_print_done_jobs(context: &mut ShellContext, stdout: &mut dyn Write) -> Result<(), String> {
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

fn main() {
    let registry = CommandRegistry::new();
    let completions = CompletionRegistry::new();
    let mut context = ShellContext::new(completions.clone());

    // Load history from HISTFILE if it exists
    let histfile = std::env::var("HISTFILE").ok();
    if let Some(path) = &histfile {
        let _ = context.load_history_from_file(path);
    }

    let config = Config::builder()
        .completion_type(CompletionType::List)
        .bell_style(BellStyle::Audible)
        .build();
    let mut editor = Editor::<ShellAutocomplete, DefaultHistory>::with_config(config).unwrap();
    editor.set_helper(Some(ShellAutocomplete::new(completions)));

    loop {
        let mut stdout = io::stdout().lock();
        let _ = reap_and_print_done_jobs(&mut context, &mut stdout);
        drop(stdout);

        let input = match editor.readline("$ ") {
            Ok(input) => input,
            Err(ReadlineError::Eof) => break,
            Err(ReadlineError::Interrupted) => {
                println!();
                continue;
            }
            Err(error) => {
                let mut stderr = io::stderr().lock();
                writeln!(stderr, "{error}").unwrap();
                break;
            }
        };

        let tokens = Lexer::tokenize(&input);
        if tokens.is_empty() {
            continue;
        }

        let ast = match Parser::parse(tokens) {
            Ok(ast) => ast,
            Err(error) => {
                let mut stderr = io::stderr().lock();
                writeln!(stderr, "{error}").unwrap();
                context.previous_exit_code = 1;
                continue;
            }
        };

        // Add command to history (both shell history and rustyline history)
        context.history.push(input.clone());
        let _ = editor.add_history_entry(input.as_str());

        let mut stdout = io::stdout().lock();
        let mut stderr = io::stderr().lock();

        if let Err(error) = execute_ast(&ast, &registry, &mut context, &mut stdout, &mut stderr) {
            if !error.is_empty() {
                writeln!(stderr, "{error}").unwrap();
            }
            context.previous_exit_code = 1;
            drop(stderr);
            drop(stdout);
            
            // Check for completed jobs after command execution
            let mut stdout = io::stdout().lock();
            let _ = reap_and_print_done_jobs(&mut context, &mut stdout);
            drop(stdout);
            continue;
        }

        context.previous_exit_code = 0;
        drop(stderr);
        drop(stdout);
        
        // Check for completed jobs after command execution
        let mut stdout = io::stdout().lock();
        let _ = reap_and_print_done_jobs(&mut context, &mut stdout);
        drop(stdout);
    }

    // Save history to HISTFILE on exit
    if let Some(path) = histfile {
        let _ = context.save_history_to_file(&path);
    }
}
