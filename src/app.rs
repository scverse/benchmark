use std::sync::Arc;

use axum::{response::IntoResponse, routing::post, Router};
use axum_github_webhook_extract::{GithubEvent, GithubToken};
use serde::Deserialize;
use tower_http::trace::TraceLayer;

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum Event {
    Enqueue { repo: String, branch: String },
}

async fn handle(GithubEvent(e): GithubEvent<Event>) -> impl IntoResponse {
    match e {
        Event::Enqueue { repo, branch } => format!("repo: {repo}, branch: {branch}"),
    }
}

pub(crate) fn app() -> Result<Router, Box<dyn std::error::Error>> {
    let token = std::env::var("SECRET_TOKEN")?;
    Ok(Router::new()
        .route("/", post(handle))
        .layer(TraceLayer::new_for_http())
        .with_state(GithubToken(Arc::new(token))))
}
