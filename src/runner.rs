use std::{
    collections::VecDeque,
    error::Error,
    sync::{Arc, Mutex},
};

use crate::app::Event;

pub(crate) async fn runner(events: Arc<Mutex<VecDeque<Event>>>) -> Result<(), Box<dyn Error>> {
    loop {
        while let Some(event) = events.lock().expect("mutex was poisoned").pop_front() {
            tracing::info!("event: {:?}", event);
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}
