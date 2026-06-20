mod commands {
    pub mod cd;
    pub mod complete;
    pub mod echo;
    pub mod exit;
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
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<(), String> {
    // Extract all commands in the pipeline
    let commands = extract_pipeline_commands(node);
    if commands.is_empty() {
        return Err("empty pipeline".to_string());
    }

    // For a single command (shouldn't happen, but handle it)
    if commands.len() == 1 {
        return execute_ast(&commands[0], registry, context, stdout, stderr);
    }

    // Spawn all commands in the pipeline with pipes connecting them
    let mut children: Vec<Child> = Vec::new();

    for (i, cmd) in commands.iter().enumerate() {
        let is_first = i == 0;
        let is_last = i == commands.len() - 1;

        let mut command_obj = build_command_from_ast(cmd, registry, context)?;

        // Set up stdin from previous command's stdout
        if !is_first {
            if let Some(child) = children.last_mut() {
                if let Some(stdout_pipe) = child.stdout.take() {
                    command_obj.stdin(Stdio::from(stdout_pipe));
                }
            }
        }

        // Set up stdout for next command in pipeline or final output
        if !is_last {
            command_obj.stdout(Stdio::piped());
        }

        let child = command_obj
            .spawn()
            .map_err(|error| format!("failed to execute command: {error}"))?;

        children.push(child);
    }

    // Wait for all commands to complete
    for mut child in children {
        child
            .wait()
            .map_err(|error| format!("failed to wait for command: {error}"))?;
    }

    Ok(())
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

/// Build a Command object from an AST node without spawning it
fn build_command_from_ast(
    node: &ASTNode,
    registry: &CommandRegistry,
    _context: &ShellContext,
) -> Result<Command, String> {
    let execution = flatten_command_execution(node)?;

    match &execution.command {
        ASTNode::Command { name, args } => {
            // For now, only support external commands in pipes
            // Built-in commands in pipes would need different handling
            if name == "type" || registry.get_builtin(name).is_some() {
                return Err("built-in commands in pipes are not supported yet".to_string());
            }

            let executable_path = find_command_in_path(name)
                .ok_or_else(|| format!("{name}: not found"))?;

            let mut command = Command::new(executable_path);
            command.args(args);

            #[cfg(unix)]
            command.arg0(name);

            // Apply any redirections
            if let Some((path, append)) = execution.stdout_redirect {
                let file = open_redirect_file(&path, append)?;
                command.stdout(Stdio::from(file));
            }

            if let Some((path, append)) = execution.stderr_redirect {
                let file = open_redirect_file(&path, append)?;
                command.stderr(Stdio::from(file));
            }

            Ok(command)
        }
        _ => Err("unsupported command in pipeline".to_string()),
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

        let mut stdout = io::stdout().lock();
        let mut stderr = io::stderr().lock();

        if let Err(error) = execute_ast(&ast, &registry, &mut context, &mut stdout, &mut stderr) {
            if !error.is_empty() {
                writeln!(stderr, "{error}").unwrap();
            }
            context.previous_exit_code = 1;
            continue;
        }

        context.previous_exit_code = 0;
    }
}
