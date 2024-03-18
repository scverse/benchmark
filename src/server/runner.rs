use std::path::Path;

use anyhow::Result;
use futures::{channel::mpsc::Receiver, StreamExt};

use crate::benchmark::{asv_compare_command, sync_repo_and_run};
use crate::event::{Compare, Event};

mod comment;

pub(crate) async fn runner(mut receiver: Receiver<Event>) {
    // loop runs until sender disconnects
    while let Some(event) = receiver.next().await {
        if let Err(error) = handle_event(event).await {
            tracing::error!("{}", error);
        }
    }
}

async fn handle_event(event: Event) -> Result<()> {
    match event {
        Event::Compare(ref cmp) => {
            tracing::info!("Comparing {:?} for PR {}", cmp.run_benchmark.run_on, cmp.pr);
            let wd = sync_repo_and_run(&cmp.run_benchmark).await?;
            compare(&wd, cmp).await?;
        }
    }
    Ok(())
}

async fn compare(wd: &Path, cmp: &Compare) -> Result<()> {
    // TODO: distinguish on type level
    let [before, after] = cmp.run_benchmark.run_on.as_slice() else {
        panic!("run_on is not a slice of size 2");
    };
    let output = asv_compare_command(wd, before, after).output().await?;
    if output.status.code() != Some(0) {
        return Err(anyhow::anyhow!("asv compare exited with {}", output.status));
    }
    let table_md = String::from_utf8(output.stdout)?;
    comment::update(cmp, &table_md).await?;
    Ok(())
}
