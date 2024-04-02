use anyhow::Result;
use chrono::Utc;

use crate::constants::{is_pr_comparison, ORG, PR_COMPARISON_MARKER};
use crate::event::Compare;
use crate::octocrab_utils::PageExt;

pub(super) async fn update(cmp: &Compare, markdown: &str) -> Result<()> {
    let markdown = make(cmp, markdown);

    tracing::info!("Updating comment for {ORG}/{}’s PR {}", cmp.repo, cmp.pr);
    let github_api = octocrab::instance();
    let issue_api = github_api.issues(ORG, &cmp.repo);
    if let Some(comment) = issue_api
        .list_comments(cmp.pr)
        .send()
        .await?
        .find(&github_api, is_pr_comparison)
        .await?
    {
        issue_api.update_comment(comment.id, markdown).await?;
        tracing::info!("Updated comment at {}", comment.html_url);
    } else {
        let comment = issue_api.create_comment(cmp.pr, markdown).await?;
        tracing::info!("Created comment at {}", comment.html_url);
    }
    Ok(())
}

fn make(cmp: &Compare, markdown: &str) -> String {
    let content = if markdown.is_empty() {
        "No changes in benchmarks.".to_owned()
    } else {
        format!("## Benchmark changes\n\n{markdown}")
    };
    let Compare {
        repo,
        commits: [before, after],
        check_id,
        pr,
    } = cmp;
    let now = Utc::now();
    let t_iso = now.to_rfc3339();
    let t_human = now.to_rfc2822();
    let check_content = if let Some(check_id) = check_id {
        format!("More details: <https://github.com/scverse/benchmark/pull/{pr}/checks?check_run_id={check_id}>")
    } else {
        String::new()
    };
    format!(
        r#"
{PR_COMPARISON_MARKER}

{content}

Comparison: <https://github.com/scverse/{repo}/compare/{before}..{after}>
Last changed: <time datetime="{t_iso}">{t_human}</time>
{check_content}
"#,
    )
}
