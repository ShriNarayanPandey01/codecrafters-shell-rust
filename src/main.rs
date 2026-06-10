#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    // TODO: Uncomment the code below to pass the first stage
    
    print!("$ ");
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    print!("{}: command not found", input);
    io::stdout().flush().unwrap();
}
