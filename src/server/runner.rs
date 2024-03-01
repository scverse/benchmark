use std::path::Path;

use anyhow::Result;
use chrono::Utc;
use futures::{channel::mpsc::Receiver, StreamExt};

use crate::benchmark::{asv_compare_command, sync_repo_and_run};
use crate::event::{Compare, Event, ORG};

pub(crate) async fn runner(mut receiver: Receiver<Event>) -> Result<()> {
    // loop runs until sender disconnects
    while let Some(event) = receiver.next().await {
        // TODO: don’t exit loop on error
        match event {
            Event::Compare(ref cmp) => {
                tracing::info!("Comparing {:?} for PR {}", cmp.run_benchmark.run_on, cmp.pr);
                let wd = sync_repo_and_run(&cmp.run_benchmark).await?;
                compare(&wd, cmp).await?;
            }
        }
    }
    Ok(())
}

async fn compare(wd: &Path, cmp: &Compare) -> Result<()> {
    // TODO: distinguish on type level
    let [before, after] = cmp.run_benchmark.run_on.as_slice() else {
        unreachable!()
    };
    let output = asv_compare_command(wd, before, after)
        .spawn()?
        .wait_with_output()
        .await?;
    if output.status.code() != Some(0) {
        return Err(anyhow::anyhow!("asv compare exited with {}", output.status));
    }
    let table_md = String::from_utf8(output.stdout)?;
    update_comment(cmp, &table_md).await?;
    Ok(())
}

async fn update_comment(cmp: &Compare, markdown: &str) -> Result<()> {
    // TODO: as above
    let [_before, after] = cmp.run_benchmark.run_on.as_slice() else {
        unreachable!()
    };
    let markdown = make_comment(&cmp.run_benchmark.repo, after, markdown);
    tracing::info!("Updating comment: {markdown}");
    // TODO: update instead of spamming
    octocrab::instance()
        .issues(ORG, &cmp.run_benchmark.repo)
        .create_comment(cmp.pr, markdown)
        .await?;
    Ok(())
}

fn make_comment(repo: &str, after: &str, markdown: &str) -> String {
    let content = if markdown.is_empty() {
        "No changes in benchmarks.".to_owned()
    } else {
        format!("## Benchmark changes\n\n{markdown}")
    };
    let now = Utc::now();
    let t_iso = now.to_rfc3339();
    let t_human = now.to_rfc2822();
    format!(
        r#"
{content}

Latest commit: <https://github.com/scverse/{repo}/commit/{after}>  \n\
Last changed: <time datetime="{t_iso}">{t_human}</time>
"#,
    )
}
