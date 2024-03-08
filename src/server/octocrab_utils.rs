use std::pin::pin;

use anyhow::{bail, Result};
use futures::{future, TryStreamExt};
use octocrab::{params::repos::Reference, Page};
use serde::de::DeserializeOwned;

use crate::{cli::RunBenchmark, constants::ORG};

pub(super) trait PageExt<I>
where
    I: DeserializeOwned + 'static,
{
    async fn find<F: Fn(&I) -> bool>(
        self,
        github_api: &octocrab::Octocrab,
        pred: F,
    ) -> octocrab::Result<Option<I>>;
}

impl<I> PageExt<I> for Page<I>
where
    I: DeserializeOwned + 'static,
{
    async fn find<F: Fn(&I) -> bool>(
        self,
        github_api: &octocrab::Octocrab,
        pred: F,
    ) -> octocrab::Result<Option<I>> {
        let items = pin!(self.into_stream(github_api));
        items
            .try_filter(|item| future::ready(pred(item)))
            .try_next()
            .await
    }
}

pub(super) async fn ref_exists(
    github_client: &octocrab::Octocrab,
    RunBenchmark {
        config_ref, repo, ..
    }: &RunBenchmark,
) -> Result<bool> {
    let Some(config_ref) = config_ref.as_ref() else {
        return Ok(true);
    };
    // TODO: Once this is fixed: https://github.com/github/docs/issues/31914
    // only get_ref needs to happen
    let Err(commit_err) = github_client.commits(ORG, repo).get(config_ref).await else {
        return Ok(true);
    };
    match commit_err {
        octocrab::Error::GitHub { source, .. }
            if source.message.starts_with("No commit found for SHA") =>
        {
            tracing::info!("Failed treating {config_ref} as commit: {source:?}");
            Ok(github_client
                .repos(ORG, repo)
                .get_ref(&Reference::Commit(config_ref.to_owned()))
                .await
                .is_ok())
        }
        e => {
            bail!("API Error: {e}");
        }
    }
}
