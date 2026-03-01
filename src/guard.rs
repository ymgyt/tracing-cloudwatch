use tokio::sync::oneshot;

#[derive(Debug)]
pub(crate) struct ShutdownSignal {
    ack_tx: oneshot::Sender<()>,
}

impl ShutdownSignal {
    pub(crate) fn new() -> (Self, oneshot::Receiver<()>) {
        let (ack_tx, ack_rx) = oneshot::channel();
        (Self { ack_tx }, ack_rx)
    }

    pub(crate) fn ack(self) {
        let _ = self.ack_tx.send(());
    }
}

/// Guard returned when creating a CloudWatch layer
///
/// When this guard is dropped a shutdown signal will be
/// sent to the CloudWatch logging worker to flush logs and
/// stop processing any more logs.
///
/// This is used to ensure buffered logs are flushed on panic
/// or graceful shutdown. Use [`CloudWatchWorkerGuard::shutdown`]
/// to explicitly wait for completion.
pub struct CloudWatchWorkerGuard {
    shutdown_tx: Option<oneshot::Sender<ShutdownSignal>>,
}

impl CloudWatchWorkerGuard {
    pub(crate) fn new(shutdown_tx: oneshot::Sender<ShutdownSignal>) -> Self {
        Self {
            shutdown_tx: Some(shutdown_tx),
        }
    }

    fn take_shutdown_tx(&mut self) -> Option<oneshot::Sender<ShutdownSignal>> {
        self.shutdown_tx.take()
    }

    /// Trigger a graceful shutdown and wait for the worker to finish
    /// draining and flushing queued logs.
    pub async fn shutdown(mut self) {
        let shutdown_tx = match self.take_shutdown_tx() {
            Some(value) => value,
            None => return,
        };

        let (shutdown_signal, ack_rx) = ShutdownSignal::new();

        if shutdown_tx.send(shutdown_signal).is_err() {
            return;
        }

        _ = ack_rx.await;
    }
}

impl Drop for CloudWatchWorkerGuard {
    fn drop(&mut self) {
        let shutdown_tx = match self.take_shutdown_tx() {
            Some(value) => value,
            None => return,
        };

        let (shutdown_signal, _ack_rx) = ShutdownSignal::new();
        let _ = shutdown_tx.send(shutdown_signal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test(flavor = "current_thread")]
    async fn shutdown_waits_for_ack() {
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<ShutdownSignal>();
        let guard = CloudWatchWorkerGuard::new(shutdown_tx);

        let worker = tokio::spawn(async move {
            let signal = shutdown_rx.await.unwrap();
            sleep(Duration::from_millis(20)).await;
            signal.ack();
        });

        guard.shutdown().await;
        worker.await.unwrap();
    }
}
