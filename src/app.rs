use anyhow::{Context, Result};
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
use serde::Deserialize;
use tower_http::trace::TraceLayer;

pub(crate) const ORG: &str = "scverse";

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub(crate) enum Event {
    Enqueue { repo: String, branch: String },
}

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
    GithubEvent(event): GithubEvent<Event>,
) -> impl IntoResponse {
    match event {
        Event::Enqueue { repo, branch, .. } => handle_enqueue(repo, branch, state).await,
    }
}

async fn handle_enqueue(
    repo: String,
    branch: String,
    mut state: AppState,
) -> std::prelude::v1::Result<String, (StatusCode, String)> {
    match octocrab::instance()
        .repos(ORG, &repo)
        .get_ref(&Reference::Branch(branch.to_owned()))
        .await
    {
        Ok(_) => state
            .sender
            .send(Event::Enqueue { repo, branch })
            .await
            .map(|()| "enqueued".to_owned())
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Error: Failed to send event".to_owned(),
                )
            }),
        Err(_) => Err((
            StatusCode::BAD_REQUEST,
            format!("Error: {branch} is not a branch in {repo}."),
        )),
    }
}

pub(crate) fn app(sender: Sender<Event>) -> Result<Router> {
    let token = std::env::var("SECRET_TOKEN")
        .context("Requires the SECRET_TOKEN env variable to be set.")?;
    let state = AppState::new(GithubToken(Arc::new(token)), sender);

    Ok(Router::new()
        .route("/", post(handle))
        .layer(TraceLayer::new_for_http())
        .with_state(state))
}
