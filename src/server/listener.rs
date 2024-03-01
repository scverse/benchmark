use anyhow::Result;
use futures::{channel::mpsc::Sender, SinkExt};
use std::sync::Arc;

use axum::{
    extract::{FromRef, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use axum_github_webhook_extract::{GithubEvent, GithubToken as GitHubSecret};
use octocrab::{params::repos::Reference, Octocrab};
use tower_http::trace::TraceLayer;

use crate::event::{Event, PullRequestEvent, PullRequestEventAction, RunBenchmark, ORG};

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
    match event.action {
        PullRequestEventAction::Synchronize(sync) => {
            if event
                .pull_request
                .labels
                .iter()
                .flatten()
                .all(|e| e.name != "benchmark")
            {
                return Ok("skipped".to_owned());
            }
            let e = RunBenchmark {
                repo: event.repository.name,
                config_ref: Some(sync.after.clone()),
                run_on: vec![event.pull_request.base.sha, sync.after],
            };
            handle_enqueue(e, state).await
        }
    }
}

async fn handle_enqueue(
    req: RunBenchmark,
    mut state: AppState,
) -> Result<String, (StatusCode, String)> {
    let branch_ok = if let Some(config_ref) = &req.config_ref {
        state
            .github_client
            .repos(ORG, &req.repo)
            // TODO: check if this needs to be done differently: https://github.com/github/docs/issues/31914
            .get_ref(&Reference::Commit(config_ref.to_owned()))
            .await
            .is_ok()
    } else {
        true
    };
    if branch_ok {
        state
            .sender
            .send(req.into())
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
            format!("Error: {req} is not a valid repo/branch"),
        ))
    }
}

pub(crate) fn listen(sender: Sender<Event>, secret: &str) -> axum::Router {
    let state = AppState {
        sender,
        secret: GitHubSecret(Arc::new(secret.to_owned())),
        github_client: octocrab::instance(),
    };

    Router::new()
        .route("/", post(handle))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[cfg(test)]
mod tests;
