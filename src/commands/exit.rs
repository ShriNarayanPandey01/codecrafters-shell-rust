use std::io::Write;

use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct Exit;

impl BuiltInCommand for Exit {
    fn execute(
        &self,
        args: Vec<String>,
        _context: &mut ShellContext,
        _stdout: &mut dyn Write,
    ) -> Result<(), String> {
        let exit_code = match args.first() {
            Some(value) => value
                .parse::<i32>()
                .map_err(|_| format!("exit: {value}: numeric argument required"))?,
            None => 0,
        };

        // Save history to HISTFILE before exiting, since std::process::exit()
        // terminates immediately and skips the post-loop save in main.
        if let Ok(histfile) = std::env::var("HISTFILE") {
            let _ = _context.save_history_to_file(&histfile);
        }

        std::process::exit(exit_code);
    }
}
