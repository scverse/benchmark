mod github;

use octocrab::models::CheckRunId;

use crate::cli::RunBenchmark;

pub(crate) use github::PullRequestEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    Compare(Compare),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Compare {
    pub run_benchmark: RunBenchmark<[String; 2]>,
    pub pr: u64,
    pub check_id: Option<CheckRunId>,
}

impl From<Compare> for Event {
    fn from(c: Compare) -> Self {
        Self::Compare(c)
    }
}
