use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct Pwd;

impl BuiltInCommand for Pwd {
    fn execute(&self, _args: Vec<String>, context: &mut ShellContext) -> Result<(), String> {
        println!("{}", context.current_dir);
        Ok(())
    }
}
