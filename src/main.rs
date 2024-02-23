use anyhow::Result;

mod benchmark;
mod event;
mod repo_cache;
mod server;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    server::serve("0.0.0.0:3000").await?;
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
