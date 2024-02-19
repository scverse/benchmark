use std::sync::Arc;

use axum::{response::IntoResponse, routing::post, Router};
use axum_github_webhook_extract::{GithubEvent, GithubToken};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Event {
    action: String,
}

async fn echo(GithubEvent(e): GithubEvent<Event>) -> impl IntoResponse {
    e.action
}

pub(crate) fn app() -> Result<Router, Box<dyn std::error::Error>> {
    let token = std::env::var("GITHUB_TOKEN")?;
    Ok(Router::new()
        .route("/", post(echo))
        .with_state(GithubToken(Arc::new(token))))
}
