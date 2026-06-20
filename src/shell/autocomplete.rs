use std::collections::BTreeSet;
use std::path::Path;

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper, Result};

pub struct ShellAutocomplete {
    builtins: Vec<&'static str>,
}

impl ShellAutocomplete {
    pub fn new() -> Self {
        Self {
            builtins: vec!["echo", "exit"],
        }
    }
}

impl Helper for ShellAutocomplete {}

impl Validator for ShellAutocomplete {}

impl Highlighter for ShellAutocomplete {}

impl Hinter for ShellAutocomplete {
    type Hint = String;
}

impl Completer for ShellAutocomplete {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>)> {
        if pos != line.len() {
            return Ok((0, Vec::new()));
        }

        let prefix = match command_prefix(line, pos) {
            Some(prefix) => prefix,
            None => return Ok((0, Vec::new())),
        };

        let matches = command_matches(&self.builtins, prefix);

        Ok((0, matches))
    }
}

fn command_prefix(line: &str, pos: usize) -> Option<&str> {
    if line[..pos].contains(char::is_whitespace) {
        None
    } else {
        Some(&line[..pos])
    }
}

fn command_matches(builtins: &[&str], prefix: &str) -> Vec<Pair> {
    let mut matches = BTreeSet::new();

    for builtin in builtins {
        if builtin.starts_with(prefix) {
            matches.insert((*builtin).to_string());
        }
    }

    for executable in path_executables() {
        if executable.starts_with(prefix) {
            matches.insert(executable);
        }
    }

    let has_single_match = matches.len() == 1;
    matches
        .into_iter()
        .map(|matched| Pair {
            display: matched.clone(),
            replacement: if has_single_match {
                format!("{matched} ")
            } else {
                matched
            },
        })
        .collect()
}

fn path_executables() -> BTreeSet<String> {
    let mut executables = BTreeSet::new();
    let Some(path_var) = std::env::var_os("PATH") else {
        return executables;
    };

    for directory in std::env::split_paths(&path_var) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if is_executable_file(&path) {
                if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                    executables.insert(name.to_string());
                }
            }
        }
    }

    executables
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };

    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        true
    }
}
