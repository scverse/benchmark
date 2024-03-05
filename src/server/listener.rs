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
use octocrab::{params::repos::Reference, Octocrab};
use tower_http::trace::TraceLayer;

use crate::cli::RunBenchmark;
use crate::constants::{BENCHMARK_LABEL, ORG};
use crate::event::{Compare, Event, PullRequestEvent, PullRequestEventAction};

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
                .all(|e| e.name != BENCHMARK_LABEL)
            {
                return Ok("skipped".to_owned());
            }
            let run_benchmark = RunBenchmark {
                repo: event.repository.name,
                config_ref: Some(sync.after.clone()),
                run_on: vec![event.pull_request.base.sha, sync.after],
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
    }
}

async fn handle_enqueue(
    event: Compare,
    mut state: AppState,
) -> Result<String, (StatusCode, String)> {
    let ref_ok = if let Some(config_ref) = &event.run_benchmark.config_ref {
        // TODO: Once this is fixed: https://github.com/github/docs/issues/31914
        // only get_ref needs to happen
        let commit_res = state
            .github_client
            .commits(ORG, &event.run_benchmark.repo)
            .get(config_ref)
            .await;
        match commit_res {
            Ok(_) => true,
            Err(octocrab::Error::GitHub { source, backtrace }) => {
                tracing::error!("GitHub Error: {source}\n{backtrace}");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("GitHub Error: {source}"),
                ));
            }
            Err(e) => {
                tracing::info!("Failed treating {config_ref} as commit: {e:?}");
                state
                    .github_client
                    .repos(ORG, &event.run_benchmark.repo)
                    .get_ref(&Reference::Commit(config_ref.to_owned()))
                    .await
                    .is_ok()
            }
        }
    } else {
        true
    };
    if ref_ok {
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
