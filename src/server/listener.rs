use anyhow::Result;
use futures::{channel::mpsc::Sender, SinkExt};
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;
use tracing::Instrument;

use axum::{
    extract::{FromRef, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use axum_github_webhook_extract::{GithubEvent, GithubToken as GitHubSecret};
use octocrab::{
    models::webhook_events::payload::PullRequestWebhookEventAction as ActionType,
    params::checks::CheckRunStatus, Octocrab,
};
use tower_http::trace::TraceLayer;

use crate::constants::BENCHMARK_LABEL;
use crate::event::{Compare, Event, PullRequestEvent};
use crate::{cli::RunBenchmark, constants::ORG};

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
        repository,
        ..
    }): GithubEvent<PullRequestEvent>,
) -> impl IntoResponse {
    if !matches!(
        action,
        ActionType::Opened | ActionType::Reopened | ActionType::Synchronize | ActionType::Labeled
    ) {
        return Ok("skipped: event action".to_owned());
    }
    if pr
        .labels
        .iter()
        .flatten()
        .all(|e| e.name != BENCHMARK_LABEL)
    {
        return Ok("skipped: missing benchmark label".to_owned());
    }
    let run_benchmark = RunBenchmark {
        repo: repository.name,
        config_ref: Some(pr.head.sha.clone()),
        run_on: vec![pr.base.sha, pr.head.sha.clone()],
    };

    let github_client = octocrab::instance();
    let checks = github_client.checks(ORG, &run_benchmark.repo);
    let check_id = checks
        .create_check_run("benchmark", pr.head.sha)
        .status(CheckRunStatus::Queued)
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .id;
    handle_enqueue(
        Compare {
            run_benchmark,
            pr: pr.number,
            check_id,
        },
        state,
    )
    .instrument(tracing::info_span!("handle_enqueue"))
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
