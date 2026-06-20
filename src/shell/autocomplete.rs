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
        if pos != line.len() || line.contains(char::is_whitespace) {
            return Ok((0, Vec::new()));
        }

        let matches: Vec<Pair> = self
            .builtins
            .iter()
            .filter(|builtin| builtin.starts_with(line))
            .map(|builtin| Pair {
                display: (*builtin).to_string(),
                replacement: format!("{builtin} "),
            })
            .collect();

        Ok((0, matches))
    }
}
