mod commands;
mod engine;

mod lexers {
    pub mod lexer;
    pub mod token;
}

mod parser {
    pub mod ast;
    #[allow(clippy::module_inception)]
    pub mod parser;
}

mod registry {
    pub mod command_registry;
}

mod shell {
    pub mod autocomplete;
    pub mod built_in_command;
    pub mod completion_registry;
    pub mod shell_context;
}

mod server;

use std::io::{self, Write};
use std::path::PathBuf;

use engine::ShellEngine;
use rustyline::Editor;
use rustyline::config::{BellStyle, CompletionType, Config};
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use shell::autocomplete::ShellAutocomplete;

fn main() {
    load_env_file();

    let args: Vec<String> = std::env::args().collect();
    let (startup_path, args) = parse_startup_args(&args[1..]);

    if let Some(path) = startup_path {
        let path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .unwrap_or_else(|error| {
                    let mut stderr = io::stderr().lock();
                    let _ = writeln!(stderr, "failed to resolve --path: {error}");
                    std::process::exit(1);
                })
                .join(path)
        };

        if !path.is_dir() {
            let mut stderr = io::stderr().lock();
            let _ = writeln!(
                stderr,
                "invalid --path value: '{}' is not a directory",
                path.display()
            );
            std::process::exit(1);
        }

        let mut paths = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .collect::<Vec<_>>();

        if !paths.contains(&path) {
            paths.insert(0, path);
        }

        let path_value = std::env::join_paths(paths).unwrap_or_else(|error| {
            let mut stderr = io::stderr().lock();
            let _ = writeln!(stderr, "failed to set PATH: {error}");
            std::process::exit(1);
        });

        unsafe {
            std::env::set_var("PATH", path_value);
        }
    }

    if let Some((host, port)) = parse_server_args(&args) {
        if let Err(error) = server::run_server(&host, port) {
            let mut stderr = io::stderr().lock();
            let _ = writeln!(stderr, "{error}");
            std::process::exit(1);
        }
        return;
    }

    let engine = ShellEngine::new();
    let mut context = engine.new_context();

    let histfile = std::env::var("HISTFILE").ok();
    if let Some(path) = &histfile {
        let _ = context.load_history_from_file(path);
    }

    let config = Config::builder()
        .completion_type(CompletionType::List)
        .bell_style(BellStyle::Audible)
        .build();
    let mut editor = Editor::<ShellAutocomplete, DefaultHistory>::with_config(config).unwrap();
    editor.set_helper(Some(ShellAutocomplete::new(engine.completions())));

    let mut final_exit_code = 0;

    loop {
        let input = match editor.readline("$ ") {
            Ok(input) => input,
            Err(ReadlineError::Eof) => break,
            Err(ReadlineError::Interrupted) => {
                println!();
                continue;
            }
            Err(error) => {
                let mut stderr = io::stderr().lock();
                let _ = writeln!(stderr, "{error}");
                break;
            }
        };

        let _ = editor.add_history_entry(input.as_str());
        let result = engine.execute_line(&mut context, &input);

        let mut stdout = io::stdout().lock();
        let mut stderr = io::stderr().lock();
        if !result.stdout.is_empty() {
            let _ = write!(stdout, "{}", result.stdout);
        }
        if !result.stderr.is_empty() {
            let _ = write!(stderr, "{}", result.stderr);
        }

        if result.should_exit {
            final_exit_code = result.exit_code;
            break;
        }
    }

    if let Some(path) = histfile {
        let _ = context.save_history_to_file(&path);
    }

    if final_exit_code != 0 {
        std::process::exit(final_exit_code);
    }
}

fn parse_server_args(args: &[String]) -> Option<(String, u16)> {
    if args.is_empty() {
        return None;
    }

    if !matches!(args[0].as_str(), "serve" | "--serve") {
        return None;
    }

    let host = std::env::var("BYOSHELL_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = args
        .get(1)
        .and_then(|value| value.parse::<u16>().ok())
        .or_else(|| std::env::var("PORT").ok()?.parse::<u16>().ok())
        .unwrap_or(7878);

    Some((host, port))
}

fn parse_startup_args(args: &[String]) -> (Option<PathBuf>, Vec<String>) {
    let mut startup_path = None;
    let mut remaining_args = Vec::new();
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        if arg == "--path" {
            if let Some(value) = iter.next() {
                startup_path = Some(PathBuf::from(value));
            } else {
                let mut stderr = io::stderr().lock();
                let _ = writeln!(stderr, "missing value for --path");
                std::process::exit(1);
            }
        } else if let Some(value) = arg.strip_prefix("--path=") {
            startup_path = Some(PathBuf::from(value));
        } else {
            remaining_args.push(arg.clone());
        }
    }

    (startup_path, remaining_args)
}

fn load_env_file() {
    let Ok(contents) = std::fs::read_to_string(".env") else {
        return;
    };

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim();
        if key.is_empty() || std::env::var_os(key).is_some() {
            continue;
        }

        let value = value.trim().trim_matches('"').trim_matches('\'');
        unsafe {
            std::env::set_var(key, value);
        }
    }
}
