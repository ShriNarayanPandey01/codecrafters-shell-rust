use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper, Result};

use crate::shell::completion_registry::CompletionRegistry;

pub struct ShellAutocomplete {
    builtins: Vec<&'static str>,
    completions: CompletionRegistry,
}

impl ShellAutocomplete {
    pub fn new(completions: CompletionRegistry) -> Self {
        Self {
            builtins: vec!["cd", "complete", "echo", "exit", "pwd", "type"],
            completions,
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

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Result<(usize, Vec<Pair>)> {
        let token = token_at_cursor(line, pos);
        let matches = if token.is_command_position {
            command_matches(&self.builtins, token.text)
        } else if let Some(command_name) = command_name(line, pos) {
            completion_matches(&self.completions, &command_name, token.text)
        } else {
            path_matches(token.text)
        };

        Ok((token.start, matches))
    }
}

struct TokenAtCursor<'a> {
    start: usize,
    text: &'a str,
    is_command_position: bool,
}

fn token_at_cursor(line: &str, pos: usize) -> TokenAtCursor<'_> {
    let before_cursor = &line[..pos];
    let start = before_cursor
        .rfind(char::is_whitespace)
        .map(|index| index + 1)
        .unwrap_or(0);

    TokenAtCursor {
        start,
        text: &line[start..pos],
        is_command_position: start == 0,
    }
}

fn command_name(line: &str, pos: usize) -> Option<String> {
    line[..pos].split_whitespace().next().map(str::to_string)
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

fn completion_matches(
    completions: &CompletionRegistry,
    command_name: &str,
    prefix: &str,
) -> Vec<Pair> {
    let Some(script_path) = completions.get(command_name) else {
        return path_matches(prefix);
    };

    let output = match Command::new(script_path).output() {
        Ok(output) => output,
        Err(_) => return Vec::new(),
    };

    let candidates = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect::<Vec<_>>();

    if candidates.is_empty() {
        return Vec::new();
    }

    let has_single_match = candidates.len() == 1;
    candidates
        .into_iter()
        .map(|candidate| Pair {
            display: candidate.clone(),
            replacement: if has_single_match {
                format!("{candidate} ")
            } else {
                candidate
            },
        })
        .collect()
}

fn path_matches(prefix: &str) -> Vec<Pair> {
    let (directory_prefix, entry_prefix, search_directory) = split_path_for_completion(prefix);
    let Ok(entries) = fs::read_dir(&search_directory) else {
        return Vec::new();
    };

    let mut matches = Vec::new();
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(|name| name.to_string()) else {
            continue;
        };

        if !name.starts_with(entry_prefix) {
            continue;
        }

        let is_directory = entry
            .file_type()
            .map(|file_type| file_type.is_dir())
            .unwrap_or(false);
        matches.push(PathMatch { name, is_directory });
    }

    matches.sort_by(|left, right| left.name.cmp(&right.name));

    if matches.is_empty() {
        return Vec::new();
    }

    if matches.len() == 1 {
        let matched = &matches[0];
        let completed = format!("{directory_prefix}{}", matched.name);
        let replacement = if matched.is_directory {
            format!("{completed}/")
        } else {
            format!("{completed} ")
        };

        return vec![Pair {
            display: display_match(&matched.name, matched.is_directory),
            replacement,
        }];
    }

    let common_prefix = longest_common_prefix(
        &matches
            .iter()
            .map(|matched| matched.name.as_str())
            .collect::<Vec<_>>(),
    )
    .to_string();

    matches
        .into_iter()
        .map(|matched| Pair {
            display: display_match(&matched.name, matched.is_directory),
            replacement: format!("{directory_prefix}{common_prefix}"),
        })
        .collect()
}

struct PathMatch {
    name: String,
    is_directory: bool,
}

fn split_path_for_completion(prefix: &str) -> (String, &str, PathBuf) {
    if let Some(last_separator) = prefix.rfind('/') {
        let directory_prefix = prefix[..=last_separator].to_string();
        let entry_prefix = &prefix[last_separator + 1..];
        let search_directory = if directory_prefix == "/" {
            PathBuf::from("/")
        } else {
            PathBuf::from(directory_prefix.trim_end_matches('/'))
        };

        (directory_prefix, entry_prefix, search_directory)
    } else {
        ("".to_string(), prefix, PathBuf::from("."))
    }
}

fn display_match(name: &str, is_directory: bool) -> String {
    if is_directory {
        format!("{name}/")
    } else {
        name.to_string()
    }
}

fn longest_common_prefix<'a>(values: &[&'a str]) -> &'a str {
    let Some(first) = values.first().copied() else {
        return "";
    };

    let mut prefix_len = first.len();
    for value in values.iter().skip(1) {
        let shared_bytes = first
            .bytes()
            .zip(value.bytes())
            .take_while(|(left, right)| left == right)
            .count();
        prefix_len = prefix_len.min(shared_bytes);
    }

    &first[..prefix_len]
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
