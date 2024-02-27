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
use axum_github_webhook_extract::{GithubEvent, GithubToken};
use octocrab::params::repos::Reference;
use tower_http::trace::TraceLayer;

use crate::event::{Event, PullRequestEvent, PullRequestEventAction, RunBenchmark, ORG};

#[derive(Debug, Clone)]
struct AppState {
    token: GithubToken,
    sender: Sender<Event>,
}

impl AppState {
    fn new(token: GithubToken, sender: Sender<Event>) -> Self {
        Self { token, sender }
    }
}

impl FromRef<AppState> for GithubToken {
    fn from_ref(state: &AppState) -> GithubToken {
        state.token.clone()
    }
}

async fn handle(
    State(state): State<AppState>,
    GithubEvent(event): GithubEvent<PullRequestEvent>,
) -> impl IntoResponse {
    match event.action {
        PullRequestEventAction::Synchronize(sync) => {
            // TODO: skip if not labelled
            let e = RunBenchmark {
                repo: ORG.to_owned(),
                branch: None,
                run_on: format!("{}..{}", sync.before, sync.after).into(),
            };
            handle_enqueue(e, state).await
        }
    }
}

async fn handle_enqueue(
    req: RunBenchmark,
    mut state: AppState,
) -> Result<String, (StatusCode, String)> {
    let branch_ok = if let Some(branch) = &req.branch {
        octocrab::instance()
            .repos(ORG, &req.repo)
            .get_ref(&Reference::Branch(branch.to_owned()))
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

pub(crate) fn listen(sender: Sender<Event>, token: &str) -> Result<Router> {
    let state = AppState::new(GithubToken(Arc::new(token.to_owned())), sender);

    Ok(Router::new()
        .route("/", post(handle))
        .layer(TraceLayer::new_for_http())
        .with_state(state))
}
