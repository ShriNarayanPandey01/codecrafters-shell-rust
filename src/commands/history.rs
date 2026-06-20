use std::io::Write;

use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct History;

impl BuiltInCommand for History {
    fn execute(
        &self,
        _args: Vec<String>,
        context: &mut ShellContext,
        stdout: &mut dyn Write,
    ) -> Result<(), String> {
        for (index, command) in context.history.iter().enumerate() {
            writeln!(stdout, "{:5}  {}", index + 1, command)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}
