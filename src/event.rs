use clap::Args;
use serde::Deserialize;
use std::fmt::Display;

mod github;

pub(crate) use github::{PullRequestEvent, PullRequestEventAction};
pub(crate) const ORG: &str = "scverse";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub(crate) enum Event {
    Enqueue(RunBenchmark),
}

#[derive(Args, Debug, Clone, Deserialize, PartialEq, Eq)]
pub(crate) struct RunBenchmark {
    /// Repository containing ASV benchmarks (in scverse org)
    pub repo: String,
    /// Branch to use benchmark configuration from
    #[arg(long, short)]
    pub branch: Option<String>,
    /// Which refs in the target repository to run benchmarks on (default: default branch)
    pub run_on: Vec<String>,
}

impl From<RunBenchmark> for Event {
    fn from(val: RunBenchmark) -> Self {
        Event::Enqueue(val)
    }
}

impl Display for RunBenchmark {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{ORG}/{}", self.repo)?;
        if let Some(branch) = &self.branch {
            write!(f, "@{branch}")?;
        }
        Ok(())
    }
}
