use octocrab::models::{issues::Comment, AppId};

pub(crate) const ORG: &str = "scverse";
pub(crate) const APP_ID: AppId = AppId(858_840);
pub(crate) const BOT_NAME: &str = "scverse-benchmark[bot]";
pub(crate) const BENCHMARK_LABEL: &str = "benchmark";
pub(crate) const PR_COMPARISON_MARKER: &str =
    "<!-- DO NOT REMOVE: Scverse benchmark run comment marker -->";

pub(crate) fn is_pr_comparison(comment: &Comment) -> bool {
    comment.user.login == BOT_NAME
        && comment
            .body
            .as_ref()
            .is_some_and(|body| body.contains(PR_COMPARISON_MARKER))
}
