use std::io::Write;
use std::path::Path;

use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct Cd;

impl BuiltInCommand for Cd {
    fn execute(
        &self,
        args: Vec<String>,
        context: &mut ShellContext,
        _stdout: &mut dyn Write,
    ) -> Result<(), String> {
        let target = args
            .first()
            .ok_or_else(|| "cd: missing argument".to_string())?;
        let resolved_target = if target == "~" {
            std::env::var("HOME").map_err(|_| "cd: HOME not set".to_string())?
        } else {
            target.clone()
        };
        let path = Path::new(&resolved_target);

        std::env::set_current_dir(path)
            .map_err(|_| format!("cd: {target}: No such file or directory"))?;

        context.refresh_current_dir();
        Ok(())
    }
}
