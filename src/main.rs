use std::{
    collections::VecDeque,
    error::Error,
    sync::{Arc, Mutex},
};

use tokio::net::TcpListener;

mod app;
mod runner;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let events: Arc<Mutex<VecDeque<app::Event>>> = Default::default();
    let app = app::app(events.clone())?;
    let listener = TcpListener::bind("0.0.0.0:3000").await?;

    let _ = tokio::join!(
        // TODO: why is that move thing necessary?
        async move { axum::serve(listener, app).await },
        runner::runner(events)
    );
    Ok(())
}
