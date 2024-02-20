use anyhow::Result;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::app::Event;

pub(crate) async fn runner(events: Arc<Mutex<VecDeque<Event>>>) -> Result<()> {
    loop {
        while let Some(event) = events.lock().expect("mutex was poisoned").pop_front() {
            tracing::info!("event: {:?}", event);
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}
