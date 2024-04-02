mod github;

use octocrab::models::CheckRunId;

pub(crate) use github::PullRequestEvent;

use crate::traits::RunConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    Compare(Compare),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Compare {
    pub repo: String,
    pub commits: [String; 2],
    pub pr: u64,
    pub check_id: Option<CheckRunId>,
}

impl RunConfig for Compare {
    fn repo(&self) -> &str {
        &self.repo
    }
    fn config_ref(&self) -> Option<&str> {
        Some(self.commits[1].as_str())
    }
    fn run_on(&self) -> &[String] {
        self.commits.as_slice()
    }
}

impl From<Compare> for Event {
    fn from(c: Compare) -> Self {
        Self::Compare(c)
    }
}
