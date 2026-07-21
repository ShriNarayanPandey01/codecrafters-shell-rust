use std::fs;
use std::io::Write;

use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct Mkdir;

impl BuiltInCommand for Mkdir {
    fn execute(
        &self,
        args: Vec<String>,
        context: &mut ShellContext,
        _stdout: &mut dyn Write,
    ) -> Result<(), String> {
        if args.is_empty() {
            return Err("mkdir: missing argument".to_string());
        }

        let mut create_parents = false;
        let mut paths = Vec::new();

        for arg in args {
            match arg.as_str() {
                "-p" | "--parents" => create_parents = true,
                flag if flag.starts_with('-') => {
                    return Err(format!("mkdir: unsupported option '{flag}'"));
                }
                _ => paths.push(arg),
            }
        }

        if paths.is_empty() {
            return Err("mkdir: missing operand".to_string());
        }

        for arg in paths {
            let path = context.resolve_path(&arg);
            let result = if create_parents {
                fs::create_dir_all(&path)
            } else {
                fs::create_dir(&path)
            };
            result
                .map_err(|error| format!("mkdir: cannot create directory '{}': {}", arg, error))?;
        }

        Ok(())
    }
}
