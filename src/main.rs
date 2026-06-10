#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    // TODO: Uncomment the code below to pass the first stage
    loop{

        print!("$ ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        match input.trim() {
            "echo hello" => println!("hello"),
            "echo world" => println!("world"),
            "exit" => break,
            _ =>(),
        }
        
        println!("{}: command not found", input.trim());

    }
    
}
