use anyhow::Result;
use futures::{channel::mpsc::channel, TryFutureExt};
use std::future::IntoFuture;
use tokio::net::TcpListener;
use tokio::task::JoinSet;

mod app;
mod event;
mod git;
mod runner;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let (sender, receiver) = channel::<event::Event>(32);
    let app = app::app(sender)?;
    let listener = TcpListener::bind("0.0.0.0:3000").await?;

    let mut set: JoinSet<Result<()>> = JoinSet::new();
    set.spawn(axum::serve(listener, app).into_future().err_into());
    set.spawn(runner::runner(receiver));
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
