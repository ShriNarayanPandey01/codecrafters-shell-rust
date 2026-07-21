use std::fs;
use std::io::{Read, Write};

use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct Cat;

impl BuiltInCommand for Cat {
    fn execute(
        &self,
        args: Vec<String>,
        context: &mut ShellContext,
        stdout: &mut dyn Write,
    ) -> Result<(), String> {
        if args.is_empty() {
            return Err("cat: missing argument".to_string());
        }

        for arg in args {
            let path = context.resolve_path(&arg);
            let mut file =
                fs::File::open(&path).map_err(|error| format!("cat: {}: {}", arg, error))?;
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)
                .map_err(|error| format!("cat: {}: {}", arg, error))?;
            stdout
                .write_all(&contents)
                .map_err(|error| error.to_string())?;
        }

        Ok(())
    }
}
