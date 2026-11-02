use anyhow::Result;
use askama::Template;
use chrono::{DateTime, Utc};

use crate::constants::{is_pr_comparison, ORG, PR_COMPARISON_MARKER};
use crate::event::Compare;
use crate::octocrab_utils::PageExt;

#[tracing::instrument(skip_all)]
pub(super) async fn update(cmp: &Compare, markdown: &str, full: &str, success: bool) -> Result<()> {
    let markdown = make(cmp, markdown, &failed_benchmarks(full), success)?;

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
    success: bool,
    failed: &'a [&'a str],
}

fn make(cmp: &Compare, content: &str, failed: &[&str], success: bool) -> Result<String> {
    Ok(Comment {
        pr_comparison_marker: PR_COMPARISON_MARKER,
        content,
        cmp,
        now: Utc::now(),
        success,
        failed,
    }
    .render()?)
}

/// Names of benchmarks that failed on either side, from `asv compare` markdown table rows
/// (`| Change | Before | After | Ratio | Benchmark (Parameter) |`).
fn failed_benchmarks(table: &str) -> Vec<&str> {
    table
        .lines()
        .filter_map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            match cells.as_slice() {
                ["", _, before, after, _, name, ""]
                    if *before == "failed" || *after == "failed" =>
                {
                    Some(*name)
                }
                _ => None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use octocrab::models::CheckRunId;
    use rstest::rstest;

    #[rstest]
    fn test_make(
        #[values(true, false)] success: bool,
        #[values("", "Some | Table")] content: &str,
        #[values(None, Some(3u64.into()))] check_id: Option<CheckRunId>,
    ) {
        let cmp = Compare {
            repo: "repo2".to_owned(),
            pr: 2,
            commits: ["c".to_owned(), "d".to_owned()],
            check_id,
        };
        let markdown = make(&cmp, content, &["a.b('c')"], success).unwrap();
        assert!(markdown.contains(PR_COMPARISON_MARKER));
        assert_eq!(
            !content.is_empty(),
            markdown.contains("## Benchmark changes")
        );
        assert!(markdown.contains(content));
        assert_eq!(!success, markdown.contains("> [!WARNING]"));
        assert_eq!(!success, markdown.contains("> - `a.b('c')`"));
        assert_eq!(check_id.is_some(), markdown.contains("More details:"));
        if check_id.is_some() {
            assert!(markdown.contains(
                "More details: <https://github.com/scverse/repo2/pull/2/checks?check_run_id=3>"
            ));
        }
    }

    #[test]
    fn test_failed_benchmarks() {
        let table = "\
| Change   | Before [1a] <main>   | After [38]   | Ratio   | Benchmark (Parameter)   |
|----------|----------------------|--------------|---------|-------------------------|
|          | 6.05G                | 6.05G        | 1.00    | a.peakmem('x')          |
|          | n/a                  | n/a          | n/a     | a.time('y')             |
|          | failed               | failed       | n/a     | b.time('z')             |
| +        | 1.0s                 | failed       | n/a     | b.time('w')             |
";
        assert_eq!(failed_benchmarks(table), ["b.time('z')", "b.time('w')"]);
    }
}
