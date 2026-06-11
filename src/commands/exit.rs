use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct Exit;

impl BuiltInCommand for Exit {
    fn execute(&self, _args: Vec<String>, context: &mut ShellContext) -> Result<(), String> {
        std::process::exit(context.previous_exit_code);
    }
}
