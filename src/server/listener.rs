use anyhow::{Context, Result};
use futures::{channel::mpsc::Sender, SinkExt};
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;

use axum::{
    extract::{FromRef, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use axum_github_webhook_extract::{GithubEvent, GithubToken as GitHubSecret};
use octocrab::models::webhook_events::payload::{
    PullRequestWebhookEventAction as ActionType, PullRequestWebhookEventPayload as PullRequestEvent,
};
use octocrab::{models::Repository, params::checks::CheckRunStatus, Octocrab};
use tower_http::trace::TraceLayer;

use crate::constants::{BENCHMARK_LABEL, ORG};
use crate::event::{Compare, Event};

use super::octocrab_utils::ref_exists;

#[derive(Debug, Clone)]
struct AppState {
    sender: Sender<Event>,
    secret: GitHubSecret,
    github_client: Arc<Octocrab>,
}

impl FromRef<AppState> for GitHubSecret {
    fn from_ref(state: &AppState) -> GitHubSecret {
        state.secret.clone()
    }
}

async fn handle(
    State(state): State<AppState>,
    GithubEvent(PullRequestEvent {
        pull_request: pr,
        action,
        label,
        ..
    }): GithubEvent<PullRequestEvent>,
) -> impl IntoResponse {
    if !matches!(
        action,
        ActionType::Opened | ActionType::Reopened | ActionType::Synchronize | ActionType::Labeled
    ) {
        return Ok("skipped: event action".to_owned());
    }
    if matches!(action, ActionType::Labeled)
        && label
            .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing label".to_owned()))?
            .name
            != BENCHMARK_LABEL
    {
        return Ok("skipped: added label is not benchmark".to_owned());
    }
    if pr
        .labels
        .iter()
        .flatten()
        .all(|e| e.name != BENCHMARK_LABEL)
    {
        return Ok("skipped: missing benchmark label".to_owned());
    }
    let Some(Repository { name: repo, .. }) = pr.base.repo else {
        return Err((StatusCode::BAD_REQUEST, "missing repo".to_owned()));
    };

    let github_client = octocrab::instance();
    let checks = github_client.checks(ORG, &repo);
    // `.ok()` allows creating the check run creation to fail. We’ll not try to update it in that case.
    let check_id = checks
        .create_check_run("benchmark", &pr.head.sha)
        .status(CheckRunStatus::Queued)
        .send()
        .await
        .map(|c| c.id)
        .context("Failed to create check run")
        .map_err(|e| tracing::error!("{e:?}"))
        .ok();
    handle_enqueue(
        Compare {
            repo,
            commits: [pr.base.sha, pr.head.sha],
            pr: pr.number,
            check_id,
        },
        state,
    )
    .await
}

#[tracing::instrument(skip_all, fields(repo = %event.repo, pr = %event.pr))]
async fn handle_enqueue(
    event: Compare,
    mut state: AppState,
) -> Result<String, (StatusCode, String)> {
    let ref_exists = ref_exists(&state.github_client, &event.repo, &event.commits[1])
        .await
        .map_err(|e| {
            tracing::error!("Enqueue failed: {e:?}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;
    if ref_exists {
        state
            .sender
            .send(event.into())
            .await
            .map(|()| "enqueued".to_owned())
            .map_err(|_| {
                let msg = "Failed to send event";
                tracing::error!("Enqueue failed: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, msg.to_owned())
            })
    } else {
        let msg = format!(
            "{}/{} is not a valid repo/ref combination",
            event.repo, event.commits[1]
        );
        tracing::info!("Enqueue failed: {msg}");
        Err((StatusCode::BAD_REQUEST, msg))
    }
}

pub(crate) fn listen(sender: Sender<Event>, secret: SecretString) -> axum::Router {
    let state = AppState {
        sender,
        secret: GitHubSecret(Arc::new(secret.expose_secret().to_owned())),
        github_client: octocrab::instance(),
    };
    std::mem::drop(secret);

    Router::new()
        .route("/", post(handle))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[cfg(test)]
mod tests;
