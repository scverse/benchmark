use anyhow::Result;
use futures::{channel::mpsc::Receiver, StreamExt};

use crate::benchmark::sync_repo_and_run;
use crate::event::Event;

pub(crate) async fn runner(mut receiver: Receiver<Event>) -> Result<()> {
    // loop runs until sender disconnects
    while let Some(event) = receiver.next().await {
        match event {
            Event::Enqueue(e) => sync_repo_and_run(e).await?,
        }
    }
    Ok(())
}
