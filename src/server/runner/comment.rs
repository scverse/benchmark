use anyhow::Result;
use chrono::Utc;

use crate::constants::{is_pr_comparison, ORG, PR_COMPARISON_MARKER};
use crate::event::Compare;
use crate::octocrab_utils::PageExt;

pub(super) async fn update(cmp: &Compare, markdown: &str) -> Result<()> {
    let markdown = make(&cmp.repo, &cmp.commits[1], markdown);

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

fn make(repo: &str, after: &str, markdown: &str) -> String {
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
{PR_COMPARISON_MARKER}

{content}

Latest commit: <https://github.com/scverse/{repo}/commit/{after}>
Last changed: <time datetime="{t_iso}">{t_human}</time>
"#,
    )
}
