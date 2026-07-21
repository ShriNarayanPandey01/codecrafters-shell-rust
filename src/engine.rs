use std::collections::HashMap;

use crate::commands::execution::{execute_ast, reap_and_print_done_jobs};
use crate::lexers::lexer::Lexer;
use crate::parser::ast::ASTNode;
use crate::parser::parser::Parser;
use crate::registry::command_registry::CommandRegistry;
use crate::shell::completion_registry::CompletionRegistry;
use crate::shell::shell_context::ShellContext;

pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub should_exit: bool,
    pub current_dir: String,
}

pub struct ShellEngine {
    registry: CommandRegistry,
    completions: CompletionRegistry,
}

impl ShellEngine {
    pub fn new() -> Self {
        Self {
            registry: CommandRegistry::new(),
            completions: CompletionRegistry::new(),
        }
    }

    pub fn completions(&self) -> CompletionRegistry {
        self.completions.clone()
    }

    pub fn new_context(&self) -> ShellContext {
        ShellContext::new(self.completions())
    }

    pub fn execute_line(&self, context: &mut ShellContext, input: &str) -> ExecutionResult {
        let mut stdout_buffer = Vec::new();
        let mut stderr_buffer = Vec::new();

        let tokens = Lexer::tokenize(input);
        if tokens.is_empty() {
            return ExecutionResult {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: context.previous_exit_code,
                should_exit: context.should_exit,
                current_dir: context.current_dir.clone(),
            };
        }

        let ast = match Parser::parse(tokens) {
            Ok(ast) => ast,
            Err(error) => {
                context.previous_exit_code = 1;
                stderr_buffer.extend_from_slice(format!("{error}\n").as_bytes());
                return ExecutionResult {
                    stdout: String::new(),
                    stderr: String::from_utf8_lossy(&stderr_buffer).into_owned(),
                    exit_code: 1,
                    should_exit: false,
                    current_dir: context.current_dir.clone(),
                };
            }
        };

        let ast = expand_variables_in_ast(ast, &context.variables);
        context.history.push(input.to_string());

        if let Err(error) = execute_ast(
            &ast,
            &self.registry,
            context,
            &mut stdout_buffer,
            &mut stderr_buffer,
        ) {
            if !error.is_empty() {
                stderr_buffer.extend_from_slice(format!("{error}\n").as_bytes());
            }
            context.previous_exit_code = 1;
        } else {
            context.previous_exit_code = if context.should_exit {
                context.requested_exit_code
            } else {
                0
            };
        }

        let _ = reap_and_print_done_jobs(context, &mut stdout_buffer);

        ExecutionResult {
            stdout: String::from_utf8_lossy(&stdout_buffer).into_owned(),
            stderr: String::from_utf8_lossy(&stderr_buffer).into_owned(),
            exit_code: context.previous_exit_code,
            should_exit: context.should_exit,
            current_dir: context.current_dir.clone(),
        }
    }
}

fn expand_variable_in_string(s: &str, variables: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            if chars.peek() == Some(&'{') {
                chars.next();
                let mut var_name = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '}' {
                        chars.next();
                        break;
                    }
                    var_name.push(c);
                    chars.next();
                }
                if let Some(value) = variables.get(&var_name) {
                    result.push_str(value);
                }
            } else {
                let mut var_name = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
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
                    result.push('$');
                } else if let Some(value) = variables.get(&var_name) {
                    result.push_str(value);
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

fn expand_variables_in_ast(node: ASTNode, variables: &HashMap<String, String>) -> ASTNode {
    match node {
        ASTNode::Command { name, args } => {
            let expanded_name = expand_variable_in_string(&name, variables);
            let expanded_args: Vec<String> = args
                .into_iter()
                .map(|arg| expand_variable_in_string(&arg, variables))
                .filter(|arg| !arg.is_empty())
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
