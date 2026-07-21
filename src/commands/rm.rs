use std::fs;
use std::io::Write;

use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct Rm;

impl BuiltInCommand for Rm {
    fn execute(
        &self,
        args: Vec<String>,
        context: &mut ShellContext,
        _stdout: &mut dyn Write,
    ) -> Result<(), String> {
        if args.is_empty() {
            return Err("rm: missing argument".to_string());
        }

        let mut recursive = false;
        let mut force = false;
        let mut targets = Vec::new();

        for arg in args {
            match arg.as_str() {
                "-r" | "-R" | "--recursive" => recursive = true,
                "-f" | "--force" => force = true,
                flag if flag.starts_with('-') => {
                    return Err(format!("rm: unsupported option '{flag}'"));
                }
                _ => targets.push(arg),
            }
        }

        if targets.is_empty() {
            return Err("rm: missing operand".to_string());
        }

        for arg in targets {
            let path = context.resolve_path(&arg);
            if path.is_dir() {
                if !recursive {
                    return Err(format!("rm: cannot remove '{}': Is a directory", arg));
                }

                if let Err(error) = fs::remove_dir_all(&path) {
                    if force && !path.exists() {
                        continue;
                    }
                    return Err(format!("rm: cannot remove '{}': {}", arg, error));
                }
                continue;
            }

            if let Err(error) = fs::remove_file(&path) {
                if force && !path.exists() {
                    continue;
                }
                return Err(format!("rm: cannot remove '{}': {}", arg, error));
            }
        }

        Ok(())
    }
}
