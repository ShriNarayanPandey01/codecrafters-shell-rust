use std::io::Write;

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
            std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .map_err(|_| "cd: HOME not set".to_string())?
        } else {
            target.clone()
        };
        let absolute_path = context.resolve_path(&resolved_target);
        let canonical_path = std::fs::canonicalize(&absolute_path)
            .map_err(|_| format!("cd: {target}: No such file or directory"))?;

        if !canonical_path.is_dir() {
            return Err(format!("cd: {target}: No such file or directory"));
        }

        context.set_current_dir_path(canonical_path);
        Ok(())
    }
}
