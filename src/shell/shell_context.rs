pub struct ShellContext {
    pub current_dir: String,
    pub previous_exit_code: i32,
}

impl ShellContext {
    pub fn new() -> Self {
        Self {
            current_dir: std::env::current_dir()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            previous_exit_code: 0,
        }
    }
}
