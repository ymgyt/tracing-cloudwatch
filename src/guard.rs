use std::time::Duration;

use tokio::{
    runtime::Handle,
    sync::mpsc::{error::SendTimeoutError, Sender},
};

/// Guard returned when creating a CloudWatch layer
///
/// When this guard is dropped a shutdown signal will be
/// sent to the CloudWatch logging worker to flush logs and
/// stop processing any more logs.
///
/// This is used to ensure buffered logs are flushed on panic
/// or graceful shutdown.
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
