use std::future::Future;

use anyhow::Result;
use octocrab::{
    checks::ChecksHandler,
    models::CheckRunId,
    params::checks::{CheckRunConclusion, CheckRunOutput, CheckRunStatus},
};

use crate::server::octocrab_utils::clamp_lines;

/// Update the check run before and after the function ran.
pub(super) async fn with_check<Fut>(
    checks: ChecksHandler<'_>,
    check_id: CheckRunId,
    func: impl Fn() -> Fut,
) -> Result<String>
where
    Fut: Future<Output = Result<(String, bool)>>,
{
    checks
        .update_check_run(check_id)
        .status(CheckRunStatus::InProgress)
        .send()
        .await?;
    let mut output = CheckRunOutput {
        title: "Benchmark".to_owned(),
        summary: String::new(),
        text: None,
        annotations: vec![],
        images: vec![],
    };
    let (conclusion, res) = match func().await {
        Ok((text, success)) => {
            "Benchmark run successful".clone_into(&mut output.summary);
            output.text = Some(clamp_lines(&text, u16::MAX.into()).to_owned());
            let conclusion = if success {
                CheckRunConclusion::Success
            } else {
                CheckRunConclusion::Failure
            };
            (conclusion, Ok(text))
        }
        Err(e) => {
            "Benchmark run failed".clone_into(&mut output.summary);
            output.text = Some(format!("## Error message\n{e}"));
            (CheckRunConclusion::Failure, Err(e))
        }
    };
    checks
        .update_check_run(check_id)
        .status(CheckRunStatus::Completed)
        .conclusion(conclusion)
        .output(output)
        .send()
        .await?;
    res
}
