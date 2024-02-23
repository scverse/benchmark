use clap::Args;
use serde::Deserialize;
use std::fmt::Display;

pub(crate) const ORG: &str = "scverse";

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub(crate) enum Event {
    Enqueue(RunBenchmark),
}

#[derive(Args, Debug, Clone, Deserialize)]
pub(crate) struct RunBenchmark {
    /// Repository containing ASV benchmarks (in scverse org)
    pub repo: String,
    /// Branch to use benchmark configuration from
    #[arg(long, short, default_value = "main")] // TODO: use actual default branch
    pub branch: String,
    /// Which refs in the target repository to run benchmarks on
    #[arg(long)]
    pub run_on: Option<String>,
}

impl From<RunBenchmark> for Event {
    fn from(val: RunBenchmark) -> Self {
        Event::Enqueue(val)
    }
}

impl Display for RunBenchmark {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{ORG}/{}@{}", self.repo, self.branch)
    }
}
