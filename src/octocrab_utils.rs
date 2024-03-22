use std::pin::pin;

use futures::{future, TryStreamExt};
use octocrab::Page;
use serde::de::DeserializeOwned;

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
