use std::time::Duration;

use tokio::{
    runtime::Handle,
    sync::mpsc::{error::SendTimeoutError, Sender},
};

pub struct CloudWatchWorkerGuard {
    shutdown_tx: Sender<()>,
}

impl CloudWatchWorkerGuard {
    pub(crate) fn new(shutdown_tx: Sender<()>) -> Self {
        Self { shutdown_tx }
    }
}

impl Drop for CloudWatchWorkerGuard {
    fn drop(&mut self) {
        let handle = Handle::current();

        match handle.block_on(
            self.shutdown_tx
                .send_timeout((), Duration::from_millis(1000)),
        ) {
            Ok(_) => {}
            Err(SendTimeoutError::Closed(_)) => (),
            Err(SendTimeoutError::Timeout(e)) => println!(
                "Failed to send shutdown signal to logging worker. Error: {:?}",
                e
            ),
        }
    }
}
