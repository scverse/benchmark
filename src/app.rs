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
use tower_http::trace::TraceLayer;

use crate::event::{Enqueue, Event, ORG};

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
        Event::Enqueue(e) => handle_enqueue(e, state).await,
    }
}

async fn handle_enqueue(e: Enqueue, mut state: AppState) -> Result<String, (StatusCode, String)> {
    match octocrab::instance()
        .repos(ORG, &e.repo)
        .get_ref(&Reference::Branch(e.branch.to_owned()))
        .await
    {
        Ok(_) => state
            .sender
            .send(e.into())
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
            format!("Error: {e} is not a valid repo/branch"),
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
