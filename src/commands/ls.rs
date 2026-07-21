use std::fs;
use std::io::Write;

use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct Ls;

struct LsOptions {
    show_all: bool,
    long_format: bool,
}

impl BuiltInCommand for Ls {
    fn execute(
        &self,
        args: Vec<String>,
        context: &mut ShellContext,
        stdout: &mut dyn Write,
    ) -> Result<(), String> {
        let (options, paths) = parse_ls_args(args)?;
        let targets = if paths.is_empty() {
            vec![context.current_dir_path()]
        } else {
            paths.iter().map(|arg| context.resolve_path(arg)).collect()
        };

        for target in targets {
            if target.is_dir() {
                let mut entries = fs::read_dir(&target)
                    .map_err(|error| {
                        format!("ls: cannot access '{}': {}", target.display(), error)
                    })?
                    .filter_map(|entry| entry.ok())
                    .filter_map(|entry| {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if !options.show_all && name.starts_with('.') {
                            return None;
                        }

                        let rendered = if options.long_format {
                            render_long_entry(&entry.path(), &name)
                        } else {
                            name
                        };

                        Some(rendered)
                    })
                    .collect::<Vec<_>>();

                entries.sort();
                for entry in entries {
                    writeln!(stdout, "{entry}").map_err(|error| error.to_string())?;
                }
            } else if target.exists() {
                let file_name = target
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| target.display().to_string());
                let rendered = if options.long_format {
                    render_long_entry(&target, &file_name)
                } else {
                    file_name
                };
                writeln!(stdout, "{rendered}").map_err(|error| error.to_string())?;
            } else {
                return Err(format!(
                    "ls: cannot access '{}': No such file or directory",
                    target.display()
                ));
            }
        }

        Ok(())
    }
}

fn parse_ls_args(args: Vec<String>) -> Result<(LsOptions, Vec<String>), String> {
    let mut options = LsOptions {
        show_all: false,
        long_format: false,
    };
    let mut paths = Vec::new();

    for arg in args {
        if let Some(flags) = arg.strip_prefix('-')
            && !flags.is_empty()
        {
            for flag in flags.chars() {
                match flag {
                    'a' => options.show_all = true,
                    'l' => options.long_format = true,
                    _ => return Err(format!("ls: unsupported option '-{flag}'")),
                }
            }
            continue;
        }

        paths.push(arg);
    }

    Ok((options, paths))
}

fn render_long_entry(path: &std::path::Path, name: &str) -> String {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return name.to_string(),
    };

    let kind = if metadata.is_dir() { 'd' } else { '-' };
    format!("{kind} {:>8} {name}", metadata.len())
}
