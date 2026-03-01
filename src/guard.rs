use tokio::sync::oneshot;

/// Guard returned when creating a CloudWatch layer
///
/// When this guard is dropped a shutdown signal will be
/// sent to the CloudWatch logging worker to flush logs and
/// stop processing any more logs.
///
/// This is used to ensure buffered logs are flushed on panic
/// or graceful shutdown.
pub struct CloudWatchWorkerGuard {
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl CloudWatchWorkerGuard {
    pub(crate) fn new(shutdown_tx: oneshot::Sender<()>) -> Self {
        Self {
            shutdown_tx: Some(shutdown_tx),
        }
    }
}

impl Drop for CloudWatchWorkerGuard {
    fn drop(&mut self) {
        let shutdown_tx = match self.shutdown_tx.take() {
            Some(value) => value,
            None => return,
        };

        _ = shutdown_tx.send(());
    }
}
