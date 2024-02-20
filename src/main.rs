use anyhow::Result;
use futures::lock::Mutex;
use futures::TryFutureExt;
use std::{collections::VecDeque, future::IntoFuture, sync::Arc};
use tokio::task::JoinSet;

use tokio::net::TcpListener;

mod app;
mod git;
mod runner;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let events: Arc<Mutex<VecDeque<app::Event>>> = Default::default();
    let app = app::app(events.clone())?; // clone Arc, not data
    let listener = TcpListener::bind("0.0.0.0:3000").await?;

    let mut set: JoinSet<Result<()>> = JoinSet::new();
    set.spawn(axum::serve(listener, app).into_future().err_into());
    set.spawn(runner::runner(events));
    while let Some(res) = set.join_next().await {
        let _ = res?;
    }
    Ok(())
}

fn init_tracing() {
    use tracing::Level;
    use tracing_subscriber::prelude::*;

    let tracing_layer = tracing_subscriber::fmt::layer();
    let filter = tracing_subscriber::filter::Targets::new()
        .with_target("tower_http::trace::make_span", Level::DEBUG)
        .with_target("tower_http::trace::on_request", Level::DEBUG)
        .with_target("tower_http::trace::on_response", Level::DEBUG)
        .with_default(Level::INFO);

    tracing_subscriber::registry()
        .with(tracing_layer)
        .with(filter)
        .init();
}
