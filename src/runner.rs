use anyhow::Result;
use directories::ProjectDirs;
use futures::lock::Mutex;
use std::path::Path;
use std::{collections::VecDeque, sync::Arc, time::Duration};
use tokio::time::sleep;

use crate::app::{Event, ORG};

lazy_static::lazy_static! {
    static ref DIRS: ProjectDirs = ProjectDirs::from("org", "scverse", "scverse benchmark").expect("No Home dir");
    static ref CACHE_DIR: &'static Path = DIRS.cache_dir();
}

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
                    run(&repo, &branch).await?
                }
            }
        }
        sleep(Duration::from_millis(100)).await;
    }
}

async fn run(repo: &str, branch: &str) -> Result<()> {
    let repo = sync_repo(repo, branch).await?;
    tracing::info!("Synced repo to {:?}", repo.path());
    // TODO: run
    Ok(())
}

async fn sync_repo(repo: &str, branch: &str) -> Result<git2::Repository> {
    let path = CACHE_DIR.join(repo);
    let repo = if path.is_dir() {
        let r = git2::Repository::open(path)?;
        // TODO: fetch
        // r.set_head(branch)?; // TODO: use correct ref
        r.checkout_head(None)?;
        r
    } else {
        git2::build::RepoBuilder::new()
            .branch(branch)
            .clone(&format!("https://github.com/{ORG}/{repo}"), &path)?
    };
    Ok(repo)
}
