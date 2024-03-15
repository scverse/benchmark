use anyhow::Result;
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
use octocrab::{
    models::webhook_events::payload::PullRequestWebhookEventAction as ActionType, Octocrab,
};
use tower_http::trace::TraceLayer;

use crate::cli::RunBenchmark;
use crate::constants::BENCHMARK_LABEL;
use crate::event::{Compare, Event, PullRequestEvent};

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
    GithubEvent(event): GithubEvent<PullRequestEvent>,
) -> impl IntoResponse {
    if !matches!(
        event.action,
        ActionType::Opened | ActionType::Reopened | ActionType::Synchronize | ActionType::Labeled
    ) {
        return Ok("skipped: event action".to_owned());
    }
    if event
        .pull_request
        .labels
        .iter()
        .flatten()
        .all(|e| e.name != BENCHMARK_LABEL)
    {
        return Ok("skipped: missing benchmark label".to_owned());
    }
    let run_benchmark = RunBenchmark {
        repo: event.repository.name,
        config_ref: Some(event.pull_request.head.sha.clone()),
        run_on: [event.pull_request.base.sha, event.pull_request.head.sha],
    };
    handle_enqueue(
        Compare {
            run_benchmark,
            pr: event.pull_request.number,
        },
        state,
    )
    .await
}

async fn handle_enqueue(
    event: Compare,
    mut state: AppState,
) -> Result<String, (StatusCode, String)> {
    if ref_exists(&state.github_client, &event.run_benchmark)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    {
        state
            .sender
            .send(event.into())
            .await
            .map(|()| "enqueued".to_owned())
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Error: Failed to send event".to_owned(),
                )
            })
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Error: {} is not a valid repo/ref combination",
                event.run_benchmark
            ),
        ))
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
