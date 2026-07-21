use std::io::Write;

use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct Exit;

impl BuiltInCommand for Exit {
    fn execute(
        &self,
        args: Vec<String>,
        context: &mut ShellContext,
        _stdout: &mut dyn Write,
    ) -> Result<(), String> {
        let exit_code = match args.first() {
            Some(value) => value
                .parse::<i32>()
                .map_err(|_| format!("exit: {value}: numeric argument required"))?,
            None => 0,
        };

        context.should_exit = true;
        context.requested_exit_code = exit_code;
        Ok(())
    }
}
