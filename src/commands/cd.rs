use std::path::Path;

use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct Cd;

impl BuiltInCommand for Cd {
    fn execute(&self, args: Vec<String>, context: &mut ShellContext) -> Result<(), String> {
        let target = args.first().ok_or_else(|| "cd: missing argument".to_string())?;
        let path = Path::new(target);

        std::env::set_current_dir(path)
            .map_err(|_| format!("cd: {target}: No such file or directory"))?;

        context.refresh_current_dir();
        Ok(())
    }
}
