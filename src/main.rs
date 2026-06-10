#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    let registry = CommandRegistry::new();
    // TODO: Uncomment the code below to pass the first stage
    loop{

        print!("$ ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let mut tokens = Lexer::tokenize(&input);


        
        match tokens[0].trim() {
            "echo" => {
                if let Some(command) = registry.get_builtin("echo") {
                    command.execute(tokens[1..].iter().map(|t| t.to_string()).collect(), &mut ShellContext::new()).unwrap();
                }
            },
            "exit" => {
                if let Some(command) = registry.get_builtin("exit") {
                    command.execute(tokens[1..].iter().map(|t| t.to_string()).collect(), &mut ShellContext::new()).unwrap();
                }
            },
            "type" => {
                if let Some(command) = registry.get_builtin(tokens[1].trim()) {
                    println!("{} is a built-in command", tokens[1].trim());
                } else {
                    println!("{}: not found", tokens[1].trim());
                    
                }
            }
            _ => println!("{}: not found", tokens[0]),
        }
        

    }
    
}
