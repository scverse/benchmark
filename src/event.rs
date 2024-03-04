mod github;

use crate::cli::RunBenchmark;

pub(crate) use github::{PullRequestEvent, PullRequestEventAction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    Compare(Compare),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Compare {
    pub run_benchmark: RunBenchmark,
    pub pr: u64,
}

impl From<Compare> for Event {
    fn from(c: Compare) -> Self {
        Self::Compare(c)
    }
}
