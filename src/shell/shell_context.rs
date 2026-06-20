use std::process::Child;

use crate::shell::completion_registry::CompletionRegistry;

pub struct BackgroundJob {
    pub job_id: usize,
    pub command: String,
    pub child: Child,
}

pub enum BackgroundJobStatus {
    Running(usize, String),
    Done(usize, String),
}

pub struct ShellContext {
    pub current_dir: String,
    pub previous_exit_code: i32,
    pub completions: CompletionRegistry,
    pub background_jobs: Vec<BackgroundJob>,
}

impl ShellContext {
    pub fn new(completions: CompletionRegistry) -> Self {
        Self {
            current_dir: current_dir_string(),
            previous_exit_code: 0,
            completions,
            background_jobs: Vec::new(),
        }
    }

    pub fn refresh_current_dir(&mut self) {
        self.current_dir = current_dir_string();
    }

    pub fn next_job_id(&self) -> usize {
        self.background_jobs
            .iter()
            .map(|job| job.job_id)
            .max()
            .map(|max_job| max_job + 1)
            .unwrap_or(1)
    }

    pub fn add_background_job(&mut self, child: Child, command: String) -> usize {
        let job_id = self.next_job_id();
        self.background_jobs.push(BackgroundJob {
            job_id,
            command,
            child,
        });
        job_id
    }

    pub fn collect_job_statuses(&mut self) -> Vec<BackgroundJobStatus> {
        let mut statuses = Vec::new();
        let mut remaining_jobs = Vec::new();

        for job in self.background_jobs.drain(..) {
            match job.child.try_wait() {
                Ok(Some(_status)) => {
                    statuses.push(BackgroundJobStatus::Done(job.job_id, job.command));
                }
                Ok(None) => {
                    statuses.push(BackgroundJobStatus::Running(job.job_id, job.command.clone()));
                    remaining_jobs.push(job);
                }
                Err(_) => {
                    statuses.push(BackgroundJobStatus::Running(job.job_id, job.command.clone()));
                    remaining_jobs.push(job);
                }
            }
        }

        self.background_jobs = remaining_jobs;
        statuses.sort_by_key(|status| match status {
            BackgroundJobStatus::Running(job_id, _) => *job_id,
            BackgroundJobStatus::Done(job_id, _) => *job_id,
        });
        statuses
    }
}

fn current_dir_string() -> String {
    std::env::current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_default()
}

fn current_dir_string() -> String {
    std::env::current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_default()
}
