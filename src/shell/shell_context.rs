pub struct ShellContext {
    pub current_dir: String,
    pub previous_exit_code: i32,
}

impl ShellContext {
    pub fn new() -> Self {
        Self {
            current_dir: current_dir_string(),
            previous_exit_code: 0,
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
