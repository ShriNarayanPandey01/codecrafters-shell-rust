use std::io::Write;

use crate::shell::shell_context::ShellContext;

pub trait BuiltInCommand {
    fn execute(
        &self,
        args: Vec<String>,
        context: &mut ShellContext,
        stdout: &mut dyn Write,
    ) -> Result<(), String>;
}
