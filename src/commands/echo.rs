use std::io::Write;

use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct Echo;

impl BuiltInCommand for Echo {
    fn execute(
        &self,
        args: Vec<String>,
        _context: &mut ShellContext,
        stdout: &mut dyn Write,
    ) -> Result<(), String> {
        let mut interpret_escapes = false;
        let mut trailing_newline = true;
        let mut start_index = 0;

        while let Some(flag) = args.get(start_index) {
            match flag.as_str() {
                "-e" => interpret_escapes = true,
                "-n" => trailing_newline = false,
                _ => break,
            }
            start_index += 1;
        }

        let output = args[start_index..].join(" ");
        let output = if interpret_escapes {
            unescape_echo_sequences(&output)
        } else {
            output
        };

        if trailing_newline {
            writeln!(stdout, "{output}").map_err(|error| error.to_string())?;
        } else {
            write!(stdout, "{output}").map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

fn unescape_echo_sequences(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }

        match chars.next() {
            Some('n') => result.push('\n'),
            Some('t') => result.push('\t'),
            Some('r') => result.push('\r'),
            Some('\\') => result.push('\\'),
            Some('0') => result.push('\0'),
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }

    result
}
