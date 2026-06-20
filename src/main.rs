mod commands {
    pub mod cd;
    pub mod echo;
    pub mod exit;
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
    pub mod built_in_command;
    pub mod shell_context;
}

use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::os::unix::process::CommandExt;

use lexers::lexer::Lexer;
use parser::ast::{ASTNode, RedirectStream};
use parser::parser::Parser;
use registry::command_registry::CommandRegistry;
use shell::shell_context::ShellContext;

fn execute_ast(
    node: &ASTNode,
    registry: &CommandRegistry,
    context: &mut ShellContext,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<(), String> {
    if matches!(node, ASTNode::Pipe { .. }) {
        return Err("pipes are parsed but not executed yet".to_string());
    }

    let execution = flatten_command_execution(node)?;
    let mut redirected_stdout = match execution.stdout_file.as_deref() {
        Some(path) => Some(File::create(path).map_err(|error| error.to_string())?),
        None => None,
    };
    let mut redirected_stderr = match execution.stderr_file.as_deref() {
        Some(path) => Some(File::create(path).map_err(|error| error.to_string())?),
        None => None,
    };

    let command_result = match execution.command {
        ASTNode::Command { name, args } => {
            if name == "type" || registry.get_builtin(name).is_some() {
                let stdout_writer = redirected_stdout
                    .as_mut()
                    .map(|file| file as &mut dyn Write)
                    .unwrap_or(stdout);
                execute_command(name, args, registry, context, stdout_writer, None, None)
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
        _ => Err("unsupported command".to_string()),
    };

    match command_result {
        Err(error) if execution.stderr_file.is_some() => {
            if !error.is_empty() {
                let path = execution.stderr_file.as_deref().unwrap();
                let mut error_file = File::create(path).map_err(|write_error| write_error.to_string())?;
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

struct CommandExecution<'a> {
    command: &'a ASTNode,
    stdout_file: Option<String>,
    stderr_file: Option<String>,
}

fn flatten_command_execution(node: &ASTNode) -> Result<CommandExecution<'_>, String> {
    let mut current = node;
    let mut stdout_file = None;
    let mut stderr_file = None;

    loop {
        match current {
            ASTNode::Redirect {
                command,
                file,
                stream,
            } => {
                match stream {
                    RedirectStream::Stdout => stdout_file = Some(file.clone()),
                    RedirectStream::Stderr => stderr_file = Some(file.clone()),
                }
                current = command;
            }
            ASTNode::Command { .. } => {
                return Ok(CommandExecution {
                    command: current,
                    stdout_file,
                    stderr_file,
                });
            }
            ASTNode::Pipe { .. } => return Err("pipes are parsed but not executed yet".to_string()),
        }
    }
}

fn main() {
    let registry = CommandRegistry::new();
    let mut context = ShellContext::new();

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

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
