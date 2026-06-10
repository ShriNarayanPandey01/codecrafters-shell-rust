pub struct exit;

impl BuiltInCommand for exit {
    fn execute(&self, _args: Vec<String>, context: &mut ShellContext) -> Result<(), String> {
        std::process::exit(context.previous_exit_code);
    }
}