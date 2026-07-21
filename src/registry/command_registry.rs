use std::collections::HashMap;

use crate::commands::cd::Cd;
use crate::commands::complete::Complete;
use crate::commands::declare::Declare;
use crate::commands::echo::Echo;
use crate::commands::exit::Exit;
use crate::commands::history::History;
use crate::commands::pwd::Pwd;
use crate::shell::built_in_command::BuiltInCommand;

pub struct CommandRegistry {
    builtins: HashMap<String, Box<dyn BuiltInCommand>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        let mut registry = CommandRegistry {
            builtins: HashMap::new(),
        };

        registry.register_builtin("cat".to_string(), Box::new(crate::commands::cat::Cat));
        registry.register_builtin("cd".to_string(), Box::new(Cd));
        registry.register_builtin("complete".to_string(), Box::new(Complete));
        registry.register_builtin("declare".to_string(), Box::new(Declare));
        registry.register_builtin("echo".to_string(), Box::new(Echo));
        registry.register_builtin("exit".to_string(), Box::new(Exit));
        registry.register_builtin("history".to_string(), Box::new(History));
        registry.register_builtin("jobs".to_string(), Box::new(crate::commands::jobs::Jobs));
        registry.register_builtin("ls".to_string(), Box::new(crate::commands::ls::Ls));
        registry.register_builtin("mkdir".to_string(), Box::new(crate::commands::mkdir::Mkdir));
        registry.register_builtin("pwd".to_string(), Box::new(Pwd));
        registry.register_builtin("rm".to_string(), Box::new(crate::commands::rm::Rm));
        registry.register_builtin("touch".to_string(), Box::new(crate::commands::touch::Touch));

        registry
    }

    pub fn register_builtin(&mut self, name: String, command: Box<dyn BuiltInCommand>) {
        self.builtins.insert(name, command);
    }

    pub fn get_builtin(&self, name: &str) -> Option<&dyn BuiltInCommand> {
        self.builtins.get(name).map(Box::as_ref)
    }
}
