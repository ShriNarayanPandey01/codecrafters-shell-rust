use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Clone, Default)]
pub struct CompletionRegistry {
    specs: Rc<RefCell<HashMap<String, String>>>,
    disabled: Rc<RefCell<HashSet<String>>>,
}

impl CompletionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, command: String, script_path: String) {
        self.disabled.borrow_mut().remove(&command);
        self.specs.borrow_mut().insert(command, script_path);
    }

    pub fn get(&self, command: &str) -> Option<String> {
        self.specs.borrow().get(command).cloned()
    }

    pub fn remove(&self, command: &str) {
        self.specs.borrow_mut().remove(command);
        self.disabled.borrow_mut().insert(command.to_string());
    }

    pub fn is_disabled(&self, command: &str) -> bool {
        self.disabled.borrow().contains(command)
    }

    pub fn entries(&self) -> Vec<(String, String)> {
        let mut entries = self
            .specs
            .borrow()
            .iter()
            .map(|(command, script)| (command.clone(), script.clone()))
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        entries
    }
}
