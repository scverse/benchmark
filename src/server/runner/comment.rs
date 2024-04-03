use anyhow::Result;
use askama::Template;
use chrono::{DateTime, Utc};

use crate::constants::{is_pr_comparison, ORG, PR_COMPARISON_MARKER};
use crate::event::Compare;
use crate::octocrab_utils::PageExt;

pub(super) async fn update(cmp: &Compare, markdown: &str) -> Result<()> {
    let markdown = make(cmp, markdown)?;

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

#[derive(Template)]
#[template(path = "comment.md.j2", escape = "none")]
struct Comment<'a> {
    pr_comparison_marker: &'a str,
    content: &'a str,
    now: DateTime<Utc>,
    cmp: &'a Compare,
}

fn make(cmp: &Compare, content: &str) -> Result<String> {
    Ok(Comment {
        pr_comparison_marker: PR_COMPARISON_MARKER,
        content,
        cmp,
        now: Utc::now(),
    }
    .render()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_empty() {
        let cmp = Compare {
            repo: "repo1".to_owned(),
            pr: 1,
            commits: ["a".to_owned(), "b".to_owned()],
            check_id: None,
        };
        let markdown = make(&cmp, "").unwrap();
        assert!(markdown.contains(PR_COMPARISON_MARKER));
        assert!(!markdown.contains("## Benchmark changes"));
        assert!(markdown.contains("No changes in benchmarks."));
        assert!(!markdown.contains("More details:"));
    }

    #[test]
    fn test_make_filled() {
        let cmp = Compare {
            repo: "repo2".to_owned(),
            pr: 2,
            commits: ["c".to_owned(), "d".to_owned()],
            check_id: Some(3.into()),
        };
        let content = "Some | table";
        let markdown = make(&cmp, content).unwrap();
        assert!(markdown.contains(PR_COMPARISON_MARKER));
        assert!(markdown.contains("## Benchmark changes"));
        assert!(markdown.contains(content));
        assert!(markdown.contains(
            "More details: <https://github.com/scverse/repo2/pull/2/checks?check_run_id=3>"
        ));
    }
}
