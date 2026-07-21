use std::fs::OpenOptions;
use std::io::Write;

use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct Touch;

impl BuiltInCommand for Touch {
    fn execute(
        &self,
        args: Vec<String>,
        context: &mut ShellContext,
        _stdout: &mut dyn Write,
    ) -> Result<(), String> {
        if args.is_empty() {
            return Err("touch: missing file operand".to_string());
        }

        for arg in args {
            let path = context.resolve_path(&arg);
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(|error| format!("touch: cannot touch '{}': {}", arg, error))?;
        }

        Ok(())
    }
}
