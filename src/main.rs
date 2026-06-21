mod commands;

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

use std::io::{self, Write};

use commands::execution::{execute_ast, reap_and_print_done_jobs};
use lexers::lexer::Lexer;
use parser::ast::ASTNode;
use parser::parser::Parser;
use registry::command_registry::CommandRegistry;
use rustyline::Editor;
use rustyline::config::{BellStyle, CompletionType, Config};
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use shell::autocomplete::ShellAutocomplete;
use shell::completion_registry::CompletionRegistry;
use shell::shell_context::ShellContext;
use std::collections::HashMap;

/// Expand `$VAR` and `${VAR}` references in a string using the given variables map.
fn expand_variable_in_string(s: &str, variables: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            if chars.peek() == Some(&'{') {
                // ${VAR} form
                chars.next(); // consume '{'
                let mut var_name = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '}' {
                        chars.next(); // consume '}'
                        break;
                    }
                    var_name.push(c);
                    chars.next();
                }
                if let Some(value) = variables.get(&var_name) {
                    result.push_str(value);
                }
                // If not set, expands to empty string (push nothing)
            } else {
                // $VAR form — name is [a-zA-Z_][a-zA-Z0-9_]*
                let mut var_name = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        // First char must be alpha or underscore
                        if var_name.is_empty() && c.is_ascii_digit() {
                            break;
                        }
                        var_name.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if var_name.is_empty() {
                    // Bare '$' with no valid name following — keep it literal
                    result.push('$');
                } else if let Some(value) = variables.get(&var_name) {
                    result.push_str(value);
                }
                // If not set, expands to empty string (push nothing)
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Recursively expand variables in an AST node.
fn expand_variables_in_ast(node: ASTNode, variables: &HashMap<String, String>) -> ASTNode {
    match node {
        ASTNode::Command { name, args } => {
            let expanded_name = expand_variable_in_string(&name, variables);
            let expanded_args: Vec<String> = args
                .into_iter()
                .map(|a| expand_variable_in_string(&a, variables))
                .filter(|a| !a.is_empty())
                .collect();
            ASTNode::Command {
                name: expanded_name,
                args: expanded_args,
            }
        }
        ASTNode::Pipe { left, right } => ASTNode::Pipe {
            left: Box::new(expand_variables_in_ast(*left, variables)),
            right: Box::new(expand_variables_in_ast(*right, variables)),
        },
        ASTNode::Redirect {
            command,
            file,
            stream,
        } => ASTNode::Redirect {
            command: Box::new(expand_variables_in_ast(*command, variables)),
            file: expand_variable_in_string(&file, variables),
            stream,
        },
        ASTNode::Background { command } => ASTNode::Background {
            command: Box::new(expand_variables_in_ast(*command, variables)),
        },
    }
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

        // Expand $VAR and ${VAR} references before execution
        let ast = expand_variables_in_ast(ast, &context.variables);

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
