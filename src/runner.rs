use anyhow::Result;
use futures::{channel::mpsc::Receiver, StreamExt};

use crate::app::{Event, ORG};
use crate::git::sync_repo;

pub(crate) async fn runner(mut receiver: Receiver<Event>) -> Result<()> {
    // loop runs until sender disconnects
    while let Some(event) = receiver.next().await {
        match event {
            Event::Enqueue { repo, branch } => {
                tracing::info!("Handling request for {ORG}/{repo}@{branch}");
                run(repo, branch).await?
            }
        }
    }
    Ok(())
}

async fn run(repo: String, branch: String) -> Result<()> {
    let repo = tokio::task::spawn_blocking(move || sync_repo(&repo, &branch)).await??;
    tracing::info!("Synced repo to {:?}", repo.path());
    // TODO: run
    Ok(())
}
