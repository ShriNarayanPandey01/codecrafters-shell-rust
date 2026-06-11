mod commands {
    pub mod echo;
    pub mod exit;
}

mod lexers {
    pub mod lexer;
    pub mod token;
}

mod registry {
    pub mod command_registry;
}

mod shell {
    pub mod built_in_command;
    pub mod shell_context;
}

use std::io::{self, Write};

use lexers::lexer::Lexer;
use lexers::token::Token;
use registry::command_registry::CommandRegistry;
use shell::shell_context::ShellContext;

fn tokens_to_args(tokens: &[Token]) -> Vec<String> {
    tokens
        .iter()
        .filter_map(|token| token.as_word().map(str::to_owned))
        .collect()
}

fn main() {
    let registry = CommandRegistry::new();

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let tokens = Lexer::tokenize(&input);
        if tokens.is_empty() {
            continue;
        }

        let Some(command_name) = tokens[0].as_word() else {
            println!("unsupported command syntax");
            continue;
        };

        let args = tokens_to_args(&tokens[1..]);

        match command_name {
            "echo" | "exit" => {
                if let Some(command) = registry.get_builtin(command_name) {
                    command.execute(args, &mut ShellContext::new()).unwrap();
                }
            }
            "type" => {
                if let Some(arg) = tokens.get(1).and_then(Token::as_word) {
                    if registry.get_builtin(arg).is_some() {
                        println!("{arg} is a shell builtin");
                    } else {
                        println!("{arg}: not found");
                    }
                }
            }
            _ => println!("{command_name}: not found"),
        }
    }
}
