use std::io::Write;

use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct Pwd;

impl BuiltInCommand for Pwd {
    fn execute(
        &self,
        _args: Vec<String>,
        context: &mut ShellContext,
        stdout: &mut dyn Write,
    ) -> Result<(), String> {
        writeln!(stdout, "{}", context.current_dir).map_err(|error| error.to_string())?;
        Ok(())
    }
}
