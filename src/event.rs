use serde::Deserialize;

mod github;

use crate::cli::RunBenchmark;

pub(crate) use github::{PullRequestEvent, PullRequestEventAction};
pub(crate) const ORG: &str = "scverse";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub(crate) enum Event {
    Enqueue(RunBenchmark),
}

impl From<RunBenchmark> for Event {
    fn from(val: RunBenchmark) -> Self {
        Event::Enqueue(val)
    }
}
