use octocrab::models::{orgs::Organization, pulls::PullRequest, Author, Repository};
use serde::{Deserialize, Serialize};

/// A stripped down version of [`octocrab::models::webhook_events::WebhookEvent`].
/// When used in a [`axum::extract::FromRequest`] extractor, it will only match PR events.
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct PullRequestEvent {
    /// The action this event represents.
    #[serde(flatten)]
    pub action: PullRequestEventAction,
    /// The pull request number this event corresponds to.
    pub number: u64,
    /// The organization the repository belongs to
    pub organization: Organization, // actually Option<> but we only use it for scverse
    /// The repository this event corresponds to
    pub repository: Repository,
    /// The pull request this event corresponds to
    pub pull_request: PullRequest,
    /// The sender of the event
    pub sender: Author,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "action", rename_all = "snake_case")]
pub(crate) enum PullRequestEventAction {
    Synchronize(Synchronize),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Synchronize {
    pub before: String,
    pub after: String,
}

#[cfg(test)]
mod tests {
    use crate::fixtures::PR;

    use super::*;

    #[test]
    fn test_deserialize() {
        let event = serde_json::from_str::<PullRequestEvent>(PR).unwrap();
        let PullRequestEventAction::Synchronize(Synchronize { before, after }) = event.action;
        assert_eq!(before, "96180e4a5fa4dc9ada3114c831a1aa8b2fd5a1f2");
        assert_eq!(after, "0d41f8596349daeadaa17c551fa0598f0a95666d");
    }
}
