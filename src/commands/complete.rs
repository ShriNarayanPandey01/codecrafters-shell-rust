use std::io::Write;

use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct Complete;

impl BuiltInCommand for Complete {
    fn execute(
        &self,
        args: Vec<String>,
        context: &mut ShellContext,
        stdout: &mut dyn Write,
    ) -> Result<(), String> {
        match args.as_slice() {
            [flag, command] if flag == "-p" => {
                if let Some(script_path) = context.completions.get(command) {
                    writeln!(stdout, "complete -C '{script_path}' {command}")
                        .map_err(|error| error.to_string())?;
                    Ok(())
                } else {
                    Err(format!("complete: {command}: no completion specification"))
                }
            }
            [flag, script_path, command] if flag == "-C" => {
                context
                    .completions
                    .register(command.clone(), script_path.clone());
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
