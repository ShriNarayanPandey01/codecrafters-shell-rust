use std::io::Write;

use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct History;

impl BuiltInCommand for History {
    fn execute(
        &self,
        args: Vec<String>,
        context: &mut ShellContext,
        stdout: &mut dyn Write,
    ) -> Result<(), String> {
        // Determine how many entries to show
        let num_to_show = if args.is_empty() {
            context.history.len()
        } else {
            args[0]
                .parse::<usize>()
                .map_err(|_| format!("history: {}: numeric argument required", args[0]))?
        };

        // Get the starting index based on how many to show
        let start_index = if context.history.len() > num_to_show {
            context.history.len() - num_to_show
        } else {
            0
        };

        // Print the entries
        for (i, command) in context.history.iter().enumerate().skip(start_index) {
            writeln!(stdout, "{:5}  {}", i + 1, command)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}
