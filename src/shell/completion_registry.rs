use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Default)]
pub struct CompletionRegistry {
    specs: Rc<RefCell<HashMap<String, String>>>,
}

impl CompletionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, command: String, script_path: String) {
        self.specs.borrow_mut().insert(command, script_path);
    }

    pub fn get(&self, command: &str) -> Option<String> {
        self.specs.borrow().get(command).cloned()
    }
}
