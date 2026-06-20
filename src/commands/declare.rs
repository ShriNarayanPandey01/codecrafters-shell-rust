use std::io::Write;

use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct Declare;

impl BuiltInCommand for Declare {
    fn execute(
        &self,
        args: Vec<String>,
        context: &mut ShellContext,
        stdout: &mut dyn Write,
    ) -> Result<(), String> {
        if args.is_empty() {
            // No arguments, just return
            return Ok(());
        }

        if args[0] == "-p" {
            // Print variable(s)
            if args.len() < 2 {
                return Err("declare -p: variable name required".to_string());
            }

            let var_name = &args[1];
            if let Some(value) = context.variables.get(var_name) {
                writeln!(stdout, "declare -- {}=\"{}\"", var_name, value)
                    .map_err(|e| e.to_string())?;
            } else {
                return Err(format!("declare: {}: not found", var_name));
            }
            Ok(())
        } else {
            // Parse NAME=VALUE
            let assignment = &args[0];
            if let Some(eq_pos) = assignment.find('=') {
                let name = &assignment[..eq_pos];
                let value = &assignment[eq_pos + 1..];

                // Validate variable name
                if !is_valid_identifier(name) {
                    return Err(format!("declare: `{}': not a valid identifier", assignment));
                }

                context.variables.insert(name.to_string(), value.to_string());
                Ok(())
            } else {
                Err(format!("declare: `{}': not a valid identifier", assignment))
            }
        }
    }
}

fn is_valid_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let mut chars = name.chars();
    let first = chars.next().unwrap();

    // First character must be letter or underscore
    if !first.is_alphabetic() && first != '_' {
        return false;
    }

    // Rest must be alphanumeric or underscore
    for ch in chars {
        if !ch.is_alphanumeric() && ch != '_' {
            return false;
        }
    }

    true
}
