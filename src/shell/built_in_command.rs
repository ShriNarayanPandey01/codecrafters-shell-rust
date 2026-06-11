use crate::shell::shell_context::ShellContext;

pub trait BuiltInCommand {
    fn execute(&self, args: Vec<String>, context: &mut ShellContext) -> Result<(), String>;
}
