use anyhow::{Context, Result};
use futures::lock::Mutex;
use std::{collections::VecDeque, sync::Arc};

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
    // TODO: limit queue size?
    events: Arc<Mutex<VecDeque<Event>>>,
}

impl AppState {
    fn new(token: GithubToken, events: Arc<Mutex<VecDeque<Event>>>) -> Self {
        Self { token, events }
    }
}

impl FromRef<AppState> for GithubToken {
    fn from_ref(state: &AppState) -> GithubToken {
        state.token.clone()
    }
}

async fn handle(
    State(state): State<AppState>,
    GithubEvent(ref e): GithubEvent<Event>,
) -> impl IntoResponse {
    match e {
        Event::Enqueue { repo, branch, .. } => {
            match octocrab::instance()
                .repos(ORG, repo)
                .get_ref(&Reference::Branch(branch.to_owned()))
                .await
            {
                Ok(_) => {
                    state.events.lock().await.push_back(e.clone());
                    Ok("enqueued".to_string())
                }
                Err(_) => Err((
                    StatusCode::BAD_REQUEST,
                    format!("Error: {branch} is not a branch in {repo}."),
                )),
            }
        }
    }
}

pub(crate) fn app(events: Arc<Mutex<VecDeque<Event>>>) -> Result<Router> {
    let token = std::env::var("SECRET_TOKEN")
        .context("Requires the SECRET_TOKEN env variable to be set.")?;
    let state = AppState::new(GithubToken(Arc::new(token)), events);

    Ok(Router::new()
        .route("/", post(handle))
        .layer(TraceLayer::new_for_http())
        .with_state(state))
}
