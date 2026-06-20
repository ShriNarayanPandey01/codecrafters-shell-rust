use std::fs;
use std::io::Write;

use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::ShellContext;

pub struct History;

impl BuiltInCommand for History {
    fn execute(
        &self,
        args: Vec<String>,
        context: &mut ShellContext,
        stdout: &mut dyn Write,
    ) -> Result<(), String> {
        if args.is_empty() {
            // No arguments, show all history
            return display_history(context, context.history.len(), stdout);
        }

        // Check for flags
        match args[0].as_str() {
            "-r" => {
                // Read history from file
                if args.len() < 2 {
                    return Err("history -r: filename argument required".to_string());
                }
                let path = &args[1];
                read_history_file(context, path)?;
                Ok(())
            }
            "-w" => {
                // Write history to file
                if args.len() < 2 {
                    return Err("history -w: filename argument required".to_string());
                }
                let path = &args[1];
                write_history_file(context, path)?;
                context.last_saved_history_index = context.history.len();
                Ok(())
            }
            "-a" => {
                // Append history to file
                if args.len() < 2 {
                    return Err("history -a: filename argument required".to_string());
                }
                let path = &args[1];
                append_history_file(context, path)?;
                context.last_saved_history_index = context.history.len();
                Ok(())
            }
            _ => {
                // Try to parse as numeric argument
                let num_to_show = args[0]
                    .parse::<usize>()
                    .map_err(|_| format!("history: {}: numeric argument required", args[0]))?;
                display_history(context, num_to_show, stdout)
            }
        }
    }
}

fn display_history(
    context: &ShellContext,
    num_to_show: usize,
    stdout: &mut dyn Write,
) -> Result<(), String> {
    let start_index = if context.history.len() > num_to_show {
        context.history.len() - num_to_show
    } else {
        0
    };

    for (i, command) in context.history.iter().enumerate().skip(start_index) {
        writeln!(stdout, "{:5}  {}", i + 1, command)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn read_history_file(context: &mut ShellContext, path: &str) -> Result<(), String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("history -r: {}", e.to_string()))?;

    for line in content.lines() {
        if !line.is_empty() {
            context.history.push(line.to_string());
        }
    }

    Ok(())
}

fn write_history_file(context: &ShellContext, path: &str) -> Result<(), String> {
    let history_text = context.history.join("\n");
    let content = if history_text.is_empty() {
        "\n".to_string()
    } else {
        format!("{}\n", history_text)
    };

    fs::write(path, content).map_err(|e| format!("history -w: {}", e.to_string()))?;
    Ok(())
}

fn append_history_file(context: &mut ShellContext, path: &str) -> Result<(), String> {
    // Read existing content
    let existing_content = fs::read_to_string(path).unwrap_or_default();

    // Get new commands since last save
    let new_commands: Vec<String> = context
        .history
        .iter()
        .skip(context.last_saved_history_index)
        .cloned()
        .collect();

    if new_commands.is_empty() {
        return Ok(());
    }

    let new_history_text = new_commands.join("\n");
    let new_content = if existing_content.is_empty() {
        format!("{}\n", new_history_text)
    } else if existing_content.ends_with('\n') {
        format!("{}{}\n", existing_content, new_history_text)
    } else {
        format!("{}\n{}\n", existing_content, new_history_text)
    };

    fs::write(path, new_content).map_err(|e| format!("history -a: {}", e.to_string()))?;
    Ok(())
}
