use std::fmt::Display;

use serde::Deserialize;

pub(crate) const ORG: &str = "scverse";

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub(crate) enum Event {
    Enqueue(Enqueue),
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Enqueue {
    pub repo: String,
    pub branch: String,
    pub run_on: Option<String>,
}

impl From<Enqueue> for Event {
    fn from(val: Enqueue) -> Self {
        Event::Enqueue(val)
    }
}

impl Display for Enqueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{ORG}/{}@{}", self.repo, self.branch)
    }
}
