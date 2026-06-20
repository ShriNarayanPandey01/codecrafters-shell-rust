mod commands {
    pub mod echo;
    pub mod exit;
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

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

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
        ASTNode::Command { name, args } => execute_command(name, args, registry, context),
        ASTNode::Pipe { .. } => Err("pipes are parsed but not executed yet".to_string()),
        ASTNode::Redirect { .. } => Err("redirection is not supported yet".to_string()),
    }
}

fn execute_command(
    name: &str,
    args: &[String],
    registry: &CommandRegistry,
    context: &mut ShellContext,
) -> Result<(), String> {
    if name == "type" {
        return run_type_command(args, registry);
    }

    let command = registry
        .get_builtin(name)
        .ok_or_else(|| format!("{name}: not found"))?;
    command.execute(args.to_vec(), context)
}

fn run_type_command(args: &[String], registry: &CommandRegistry) -> Result<(), String> {
    let target = args
        .first()
        .ok_or_else(|| "type: missing argument".to_string())?;

    if registry.get_builtin(target).is_some() || target == "type" {
        println!("{target} is a shell builtin");
    } else if let Some(path) = find_command_in_path(target) {
        println!("{target} is {}", path.display());
    } else {
        println!("{target}: not found");
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
