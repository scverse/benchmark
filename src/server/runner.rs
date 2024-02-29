use anyhow::Result;
use futures::{channel::mpsc::Receiver, StreamExt};

use crate::benchmark::{asv_compare_command, sync_repo_and_run};
use crate::event::Event;

pub(crate) async fn runner(mut receiver: Receiver<Event>) -> Result<()> {
    // loop runs until sender disconnects
    while let Some(event) = receiver.next().await {
        // TODO: don’t exit loop on error
        match event {
            Event::Enqueue(req) => {
                tracing::info!("Handling request for {req} on {:?}", req.run_on);
                let wd = sync_repo_and_run(req.clone()).await?;
                let [before, after] = req.run_on.as_slice() else {
                    unreachable!()
                };
                let output = asv_compare_command(&wd, before, after)
                    .spawn()?
                    .wait_with_output()
                    .await?;
                if output.status.code() != Some(0) {
                    return Err(anyhow::anyhow!("asv compare exited with {}", output.status));
                }
                let x = String::from_utf8(output.stdout)?;
                tracing::info!("asv compare output: {x}");
                // TODO: send output to GitHub
            }
        }
    }
    Ok(())
}
