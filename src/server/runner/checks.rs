use std::future::Future;

use anyhow::Result;
use octocrab::{
    checks::ChecksHandler,
    models::CheckRunId,
    params::checks::{CheckRunConclusion, CheckRunOutput, CheckRunStatus},
};

use crate::server::octocrab_utils::clamp_lines;

pub(super) async fn with_check<Fut>(
    checks: ChecksHandler<'_>,
    check_id: CheckRunId,
    func: impl Fn() -> Fut,
) -> Result<String>
where
    Fut: Future<Output = Result<String>>,
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
        Ok(text) => {
            output.summary = "Benchmark run successful".to_owned();
            output.text = Some(clamp_lines(&text, u16::MAX.into()).to_owned());
            (CheckRunConclusion::Success, Ok(text))
        }
        Err(e) => {
            output.summary = "Benchmark run failed".to_owned();
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
