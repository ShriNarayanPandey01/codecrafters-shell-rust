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
            [] => {
                for (command, script_path) in context.completions.entries() {
                    writeln!(stdout, "complete -C '{}' {}", script_path, command)
                        .map_err(|error| error.to_string())?;
                }
                Ok(())
            }
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
                let resolved_path = context.resolve_path(script_path);
                if !resolved_path.exists() {
                    return Err(format!(
                        "complete: {}: no such completion script",
                        resolved_path.display()
                    ));
                }

                context
                    .completions
                    .register(command.clone(), resolved_path.display().to_string());
                Ok(())
            }
            [flag, command] if flag == "-r" => {
                context.completions.remove(command);
                Ok(())
            }
            _ => Err(
                "usage: complete [-C script command] | [-p command] | [-r command] | [no args]"
                    .to_string(),
            ),
        }
    }
}
