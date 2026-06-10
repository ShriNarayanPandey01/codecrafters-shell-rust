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
            s if s.starts_with("echo ") => println!("{}", &s[5..]),
            "exit" => break,
            _ =>(),
        }
        
        println!("{}: command not found", input.trim());

    }
    
}
