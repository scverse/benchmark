use anyhow::Result;
use futures::TryFutureExt;
use std::{
    collections::VecDeque,
    future::IntoFuture,
    sync::{Arc, Mutex},
};
use tokio::task::JoinSet;

use tokio::net::TcpListener;

mod app;
mod runner;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

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
