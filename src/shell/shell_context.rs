use crate::shell::completion_registry::CompletionRegistry;

pub struct ShellContext {
    pub current_dir: String,
    pub previous_exit_code: i32,
    pub completions: CompletionRegistry,
}

impl ShellContext {
    pub fn new(completions: CompletionRegistry) -> Self {
        Self {
            current_dir: current_dir_string(),
            previous_exit_code: 0,
            completions,
        }
    }

    pub fn refresh_current_dir(&mut self) {
        self.current_dir = current_dir_string();
    }
}

fn current_dir_string() -> String {
    std::env::current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_default()
}
