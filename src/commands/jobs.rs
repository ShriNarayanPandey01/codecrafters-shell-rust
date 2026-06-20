use std::io::Write;

use crate::shell::built_in_command::BuiltInCommand;
use crate::shell::shell_context::{BackgroundJobStatus, ShellContext};

pub struct Jobs;

impl BuiltInCommand for Jobs {
    fn execute(
        &self,
        _args: Vec<String>,
        context: &mut ShellContext,
        stdout: &mut dyn Write,
    ) -> Result<(), String> {
        let mut statuses = context.collect_job_statuses();

        for (index, status) in statuses.iter().enumerate() {
            match status {
                BackgroundJobStatus::Running(job_id, command) => {
                    let marker = if index + 1 == statuses.len() {
                        '+'
                    } else if index + 1 == statuses.len() - 1 {
                        '-'
                    } else {
                        ' '
                    };
                    writeln!(stdout, "[{job_id}]{marker}  {:24} {command} &", "Running")
                        .map_err(|error| error.to_string())?;
                }
                BackgroundJobStatus::Done(job_id, command) => {
                    let marker = if index + 1 == statuses.len() {
                        '+'
                    } else if index + 1 == statuses.len() - 1 {
                        '-'
                    } else {
                        ' '
                    };
                    writeln!(stdout, "[{job_id}]{marker}  {:24} {command}", "Done")
                        .map_err(|error| error.to_string())?;
                }
            }
        }

        Ok(())
    }
}
