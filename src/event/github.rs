use octocrab::models::webhook_events::payload::PullRequestWebhookEventAction;
use octocrab::models::{orgs::Organization, pulls::PullRequest, Author, Repository};
use serde::{Deserialize, Serialize};

/// A stripped down version of [`octocrab::models::webhook_events::WebhookEvent`].
/// When used in a [`axum::extract::FromRequest`] extractor, it will only match PR events.
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct PullRequestEvent {
    /// The action this event represents.
    pub action: PullRequestWebhookEventAction,
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
