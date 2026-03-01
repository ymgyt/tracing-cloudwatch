use crate::{
    export::{BatchExporter, ExportConfig},
    guard::ShutdownSignal,
    CloudWatchClient,
};

use chrono::{DateTime, Utc};
use tokio::sync::{
    mpsc::{self, UnboundedSender},
    oneshot,
};

pub trait Dispatcher {
    fn dispatch(&self, input: LogEvent);
}

#[derive(Debug)]
pub struct LogEvent {
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

pub struct NoopDispatcher {}

impl Dispatcher for NoopDispatcher {
    fn dispatch(&self, _event: LogEvent) {}
}

impl NoopDispatcher {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

pub struct CloudWatchDispatcher {
    tx: UnboundedSender<LogEvent>,
}

impl CloudWatchDispatcher {
    pub(crate) fn new<C>(
        client: C,
        export_config: ExportConfig,
        shutdown_rx: oneshot::Receiver<ShutdownSignal>,
    ) -> Self
    where
        C: CloudWatchClient + Send + Sync + 'static,
    {
        // Should use bounded channel?
        let (tx, rx) = mpsc::unbounded_channel();
        let exporter = BatchExporter::new(client, export_config);

        tokio::spawn(exporter.run(rx, shutdown_rx));

        Self { tx }
    }
}

impl Dispatcher for CloudWatchDispatcher {
    fn dispatch(&self, event: LogEvent) {
        // The exporter can already be shutting down when late logs arrive.
        // Drop them instead of panicking the application.
        let _ = self.tx.send(event);
    }
}

impl std::io::Write for &NoopDispatcher {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Write for &CloudWatchDispatcher {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let timestamp = Utc::now();
        let message = String::from_utf8_lossy(buf).to_string();

        self.dispatch(LogEvent { message, timestamp });

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
