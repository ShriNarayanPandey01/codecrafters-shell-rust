use std::io::Write;

use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct Echo;

impl BuiltInCommand for Echo {
    fn execute(
        &self,
        args: Vec<String>,
        _context: &mut ShellContext,
        stdout: &mut dyn Write,
    ) -> Result<(), String> {
        writeln!(stdout, "{}", args.join(" ")).map_err(|error| error.to_string())?;
        Ok(())
    }
}
