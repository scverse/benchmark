use std::path::Path;

use anyhow::Result;
use chrono::Utc;
use futures::{channel::mpsc::Receiver, StreamExt};

use crate::benchmark::{asv_compare_command, sync_repo_and_run};
use crate::event::{Event, RunBenchmark};

pub(crate) async fn runner(mut receiver: Receiver<Event>) -> Result<()> {
    // loop runs until sender disconnects
    while let Some(event) = receiver.next().await {
        // TODO: don’t exit loop on error
        match event {
            Event::Enqueue(ref req) => {
                tracing::info!("Handling request for {req} on {:?}", req.run_on);
                let wd = sync_repo_and_run(req).await?;
                compare(&wd, req).await?;
            }
        }
    }
    Ok(())
}

async fn compare(wd: &Path, req: &RunBenchmark) -> Result<()> {
    let [before, after] = req.run_on.as_slice() else {
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
    let comment_md = make_comment(&req.repo, after, &table_md);
    update_comment(&comment_md).await?;
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

async fn update_comment(markdown: &str) -> Result<()> {
    // octocrab::instance();
    tracing::info!("Updating comment: {markdown}");
    todo!();
}
