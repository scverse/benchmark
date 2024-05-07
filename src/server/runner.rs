use anyhow::Result;
use futures::{channel::mpsc::Receiver, StreamExt};
use tracing::Instrument;

use crate::benchmark::{sync_repo_and_run, AsvCompare, RunResult};
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
            tracing::error!("Handle event error: {error:?}");
        }
    }
}

async fn handle_event(event: Event) -> Result<()> {
    match event {
        Event::Compare(ref cmp) => {
            tracing::info!("Comparing {:?} for PR {}", cmp.commits, cmp.pr);
            let github_client = octocrab::instance();
            let checks_handler = github_client.checks(ORG, &cmp.repo);
            if let Some(check_id) = cmp.check_id {
                checks::with_check(checks_handler, check_id, || full_compare(cmp)).await?;
            } else {
                full_compare(cmp).await?;
            }
        }
    }
    Ok(())
}

async fn full_compare(cmp: &Compare) -> Result<(String, bool), anyhow::Error> {
    let rr = sync_repo_and_run(cmp).await?;
    let success = rr.success;
    let output = compare(rr, cmp).await?;
    Ok((output, success))
}

async fn compare(rr: RunResult, cmp: &Compare) -> Result<String> {
    let mut compare = AsvCompare::new(&rr.wd, &cmp.commits[0], &cmp.commits[1]);
    compare.in_envs(rr.env_specs);
    // Try updating comment with short comparison
    if let Err(e) = comment::update(cmp, &compare.output().await?, rr.success)
        .instrument(tracing::info_span!("comment_update"))
        .await
    {
        tracing::error!("Update comment error: {e:?}");
    }
    // Return full comparison
    compare.only_changed(false).output().await
}
