pub struct echo;

impl BuiltInCommand for echo {
    fn execute(&self , args: Vec<String>, _context: &mut ShellContext) -> Result<(), String> {
        println!("{}", args.join(" "));
        Ok(())
    }
}