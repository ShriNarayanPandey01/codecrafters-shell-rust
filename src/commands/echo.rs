use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct Echo;

impl BuiltInCommand for Echo {
    fn execute(&self, args: Vec<String>, _context: &mut ShellContext) -> Result<(), String> {
        println!("{}", args.join(" "));
        Ok(())
    }
}
