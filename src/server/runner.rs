use std::path::Path;
use std::process::Output;

use anyhow::Result;
use futures::{channel::mpsc::Receiver, StreamExt};
use tracing::Instrument;

use crate::benchmark::{asv_compare_command, sync_repo_and_run};
use crate::constants::ORG;
use crate::event::{Compare, Event};

mod checks;
mod comment;

pub(crate) async fn runner(mut receiver: Receiver<Event>) {
    // loop runs until sender disconnects
    while let Some(event) = receiver.next().await {
        if let Err(error) = handle_event(event)
            .instrument(tracing::info_span!("handle_event"))
            .await
        {
            tracing::error!("{error}");
        }
    }
}

async fn handle_event(event: Event) -> Result<()> {
    match event {
        Event::Compare(ref cmp) => {
            tracing::info!("Comparing {:?} for PR {}", cmp.run_benchmark.run_on, cmp.pr);
            let github_client = octocrab::instance();
            let checks_handler = github_client.checks(ORG, &cmp.run_benchmark.repo);
            if let Some(check_id) = cmp.check_id {
                checks::with_check(checks_handler, check_id, || full_compare(cmp)).await?;
            } else {
                full_compare(cmp).await?;
            }
        }
    }
    Ok(())
}

async fn full_compare(cmp: &Compare) -> Result<String, anyhow::Error> {
    let wd = sync_repo_and_run(&cmp.run_benchmark).await?;
    compare(&wd, cmp).await
}

async fn compare(wd: &Path, cmp: &Compare) -> Result<String> {
    // TODO: distinguish on type level
    let [before, after] = cmp.run_benchmark.run_on.as_slice() else {
        panic!("run_on is not a slice of size 2");
    };
    let Output {
        stdout,
        stderr,
        status,
    } = asv_compare_command(wd, before, after).output().await?;
    if status.code() != Some(0) {
        return Err(anyhow::anyhow!(
            "asv compare exited with {status}:\n{}",
            String::from_utf8_lossy(&stderr)
        ));
    }
    let table_md = String::from_utf8(stdout)?;
    comment::update(cmp, &table_md).await?;
    Ok(table_md)
}
