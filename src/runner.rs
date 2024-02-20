use anyhow::Result;
use futures::lock::Mutex;
use std::{collections::VecDeque, sync::Arc, time::Duration};
use tokio::time::sleep;

use crate::app::{Event, ORG};
use crate::git::sync_repo;

pub(crate) async fn runner(events: Arc<Mutex<VecDeque<Event>>>) -> Result<()> {
    loop {
        while let Some(event) = {
            // assign guard to make sure lock is released before entring the match block
            let mut queue_guard = events.lock().await;
            queue_guard.pop_front()
        } {
            match event {
                Event::Enqueue { repo, branch } => {
                    tracing::info!("Handling request for {ORG}/{repo}@{branch}");
                    run(repo, branch).await?
                }
            }
        }
        sleep(Duration::from_millis(100)).await;
    }
}

async fn run(repo: String, branch: String) -> Result<()> {
    let repo = tokio::task::spawn_blocking(move || sync_repo(&repo, &branch)).await??;
    tracing::info!("Synced repo to {:?}", repo.path());
    // TODO: run
    Ok(())
}
