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

use std::io::{self, Write};
use std::path::PathBuf;
use std::fs::File;
use std::process::Command;
use std::process::Stdio;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::os::unix::process::CommandExt;

use lexers::lexer::Lexer;
use parser::ast::ASTNode;
use parser::parser::Parser;
use registry::command_registry::CommandRegistry;
use shell::shell_context::ShellContext;

fn execute_ast(
    node: &ASTNode,
    registry: &CommandRegistry,
    context: &mut ShellContext,
) -> Result<(), String> {
    match node {
        ASTNode::Command { name, args } => {
            let mut stdout = io::stdout().lock();
            execute_command(name, args, registry, context, &mut stdout)
        }
        ASTNode::Pipe { .. } => Err("pipes are parsed but not executed yet".to_string()),
        ASTNode::Redirect { command, file } => execute_redirect(command, file, registry, context),
    }
}

fn execute_command(
    name: &str,
    args: &[String],
    registry: &CommandRegistry,
    context: &mut ShellContext,
    stdout: &mut dyn Write,
) -> Result<(), String> {
    if name == "type" {
        return run_type_command(args, registry, stdout);
    }

    if let Some(command) = registry.get_builtin(name) {
        return command.execute(args.to_vec(), context, stdout);
    }

    let executable_path = find_command_in_path(name).ok_or_else(|| format!("{name}: not found"))?;
    run_external_command(name, args, executable_path, None)
}

fn execute_redirect(
    command: &ASTNode,
    file: &str,
    registry: &CommandRegistry,
    context: &mut ShellContext,
) -> Result<(), String> {
    let output_file = File::create(file).map_err(|error| error.to_string())?;

    match command {
        ASTNode::Command { name, args } => {
            if name == "type" || registry.get_builtin(name).is_some() {
                let mut output_file = output_file;
                execute_command(name, args, registry, context, &mut output_file)
            } else {
                let executable_path =
                    find_command_in_path(name).ok_or_else(|| format!("{name}: not found"))?;
                run_external_command(name, args, executable_path, Some(output_file))
            }
        }
        _ => Err("redirection is not supported for this command".to_string()),
    }
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
) -> Result<(), String> {
    let mut command = Command::new(executable_path);
    command.args(args);

    #[cfg(unix)]
    command.arg0(name);

    if let Some(file) = stdout_file {
        command.stdout(Stdio::from(file));
    }

    command
        .status()
        .map(|_| ())
        .map_err(|error| format!("failed to execute {name}: {error}"))
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
                eprintln!("{error}");
                context.previous_exit_code = 1;
                continue;
            }
        };

        if let Err(error) = execute_ast(&ast, &registry, &mut context) {
            eprintln!("{error}");
            context.previous_exit_code = 1;
            continue;
        }

        context.previous_exit_code = 0;
    }
}
