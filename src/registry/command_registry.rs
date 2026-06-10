

pub struct CommandRegistry {

    builtins:
        HashMap<String,
        Box<dyn BuiltinCommand>>
}

impl CommandRegistry {

    pub fn new() -> Self {
        let mut registry = CommandRegistry {
            builtins: HashMap::new(),
        };

        registry.register_builtin("echo".to_string(), Box::new(echo));
        registry.register_builtin("exit".to_string(), Box::new(exit));

        registry
    }

    pub fn register_builtin(&mut self, name: String, command: Box<dyn BuiltInCommand>) {
        self.builtins.insert(name, command);
    }

    pub fn get_builtin(&self, name: &str) -> Option<&Box<dyn BuiltInCommand>> {
        self.builtins.get(name)
    }
}