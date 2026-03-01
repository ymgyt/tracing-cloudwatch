use std::fmt::Debug;
use std::num::NonZeroUsize;
use std::time::Duration;

use tokio::{
    sync::{mpsc::UnboundedReceiver, oneshot},
    time::interval,
};

use crate::{client::NoopClient, dispatch::LogEvent, guard::ShutdownSignal, CloudWatchClient};

/// Configurations to control the behavior of exporting logs to CloudWatch.
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// The number of logs to retain in the buffer within the interval period.
    batch_size: NonZeroUsize,
    /// The interval for putting logs.
    interval: Duration,
    /// Where logs are sent.
    destination: LogDestination,
}

/// Where logs are sent.
#[derive(Debug, Clone, Default)]
pub struct LogDestination {
    /// The name of the log group.
    pub log_group_name: String,
    /// The name of the log stream.
    pub log_stream_name: String,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            batch_size: NonZeroUsize::new(5).unwrap(),
            interval: Duration::from_secs(5),
            destination: LogDestination::default(),
        }
    }
}

impl ExportConfig {
    /// Set batch size.
    pub fn with_batch_size<T>(self, batch_size: T) -> Self
    where
        T: TryInto<NonZeroUsize>,
        <T as TryInto<NonZeroUsize>>::Error: Debug,
    {
        Self {
            batch_size: batch_size
                .try_into()
                .expect("batch size must be greater than or equal to 1"),
            ..self
        }
    }

    /// Set export interval.
    pub fn with_interval(self, interval: Duration) -> Self {
        Self { interval, ..self }
    }

    /// Set log group name.
    pub fn with_log_group_name(self, log_group_name: impl Into<String>) -> Self {
        Self {
            destination: LogDestination {
                log_group_name: log_group_name.into(),
                log_stream_name: self.destination.log_stream_name,
            },
            ..self
        }
    }

    /// Set log stream name.
    pub fn with_log_stream_name(self, log_stream_name: impl Into<String>) -> Self {
        Self {
            destination: LogDestination {
                log_stream_name: log_stream_name.into(),
                log_group_name: self.destination.log_group_name,
            },
            ..self
        }
    }
}

pub(crate) struct BatchExporter<C> {
    client: C,
    queue: Vec<LogEvent>,
    config: ExportConfig,
}

impl Default for BatchExporter<NoopClient> {
    fn default() -> Self {
        Self::new(NoopClient::new(), ExportConfig::default())
    }
}

impl<C> BatchExporter<C> {
    pub(crate) fn new(client: C, config: ExportConfig) -> Self {
        Self {
            client,
            config,
            queue: Vec::new(),
        }
    }
}

impl<C> BatchExporter<C>
where
    C: CloudWatchClient + Send + Sync + 'static,
{
    pub(crate) async fn run(
        mut self,
        mut rx: UnboundedReceiver<LogEvent>,
        mut shutdown_rx: oneshot::Receiver<ShutdownSignal>,
    ) {
        let mut interval = interval(self.config.interval);
        let mut shutdown_signal = None;

        loop {
            tokio::select! {
                 _ = interval.tick() => {
                    if self.queue.is_empty() {
                        continue;
                    }
                }

                event = rx.recv() => {
                    let Some(event) = event else {
                        break;
                    };

                    self.queue.push(event);
                    if self.queue.len() < <NonZeroUsize as Into<usize>>::into(self.config.batch_size) {
                        continue
                    }
                }

                received_shutdown = &mut shutdown_rx => {
                    if let Ok(signal) = received_shutdown {
                        shutdown_signal = Some(signal);
                    }
                    while let Ok(event) = rx.try_recv() {
                        self.queue.push(event);
                    }
                    break;
                }
            }
            self.flush().await;
        }
        self.flush().await;
        if let Some(shutdown_signal) = shutdown_signal {
            shutdown_signal.ack();
        }
    }

    async fn flush(&mut self) {
        let logs: Vec<LogEvent> = Self::take_from_queue(&mut self.queue);

        if logs.is_empty() {
            return;
        }

        if let Err(err) = self
            .client
            .put_logs(self.config.destination.clone(), logs)
            .await
        {
            eprintln!(
                "[tracing-cloudwatch] Unable to put logs to cloudwatch. Error: {err:?} {:?}",
                self.config.destination
            );
        }
    }

    fn take_from_queue(queue: &mut Vec<LogEvent>) -> Vec<LogEvent> {
        if cfg!(feature = "ordered_logs") {
            let mut logs = std::mem::take(queue);
            logs.sort_by_key(|log| log.timestamp);
            logs
        } else {
            std::mem::take(queue)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use std::sync::{Arc, Mutex};
    use tokio::time::{sleep, timeout};
    use tracing_subscriber::layer::SubscriberExt;

    const ONE_DAY_NS: i64 = 86_400_000_000_000;
    const DAY_ONE: DateTime<Utc> = DateTime::from_timestamp_nanos(0 + ONE_DAY_NS);
    const DAY_TWO: DateTime<Utc> = DateTime::from_timestamp_nanos(0 + (ONE_DAY_NS * 2));
    const DAY_THREE: DateTime<Utc> = DateTime::from_timestamp_nanos(0 + (ONE_DAY_NS * 3));

    #[cfg(not(feature = "ordered_logs"))]
    #[test]
    fn does_not_order_logs_by_default() {
        let mut unordered_queue = vec![
            LogEvent {
                message: "1".to_string(),
                timestamp: DAY_ONE,
            },
            LogEvent {
                message: "3".to_string(),
                timestamp: DAY_THREE,
            },
            LogEvent {
                message: "2".to_string(),
                timestamp: DAY_TWO,
            },
        ];
        let still_unordered_queue =
            BatchExporter::<NoopClient>::take_from_queue(&mut unordered_queue);

        let mut still_unordered_queue_iter = still_unordered_queue.iter();
        assert_eq!(
            DAY_ONE,
            still_unordered_queue_iter.next().unwrap().timestamp
        );
        assert_eq!(
            DAY_THREE,
            still_unordered_queue_iter.next().unwrap().timestamp
        );
        assert_eq!(
            DAY_TWO,
            still_unordered_queue_iter.next().unwrap().timestamp
        );
    }

    #[cfg(feature = "ordered_logs")]
    mod ordering {
        use super::*;

        fn assert_is_ordered(logs: Vec<LogEvent>) {
            let mut last_timestamp = DateTime::from_timestamp_nanos(0);

            for log in logs {
                assert!(
                    log.timestamp > last_timestamp,
                    "Not true: {} > {}",
                    log.timestamp,
                    last_timestamp
                );
                last_timestamp = log.timestamp;
            }
        }

        #[test]
        fn orders_logs_when_enabled() {
            let mut unordered_queue = vec![
                LogEvent {
                    message: "1".to_string(),
                    timestamp: DAY_ONE,
                },
                LogEvent {
                    message: "3".to_string(),
                    timestamp: DAY_THREE,
                },
                LogEvent {
                    message: "2".to_string(),
                    timestamp: DAY_TWO,
                },
            ];
            let ordered_queue = BatchExporter::<NoopClient>::take_from_queue(&mut unordered_queue);
            assert_is_ordered(ordered_queue);
        }
    }

    #[derive(Clone, Default)]
    struct RecordingClient {
        logs: Arc<Mutex<Vec<LogEvent>>>,
    }

    #[async_trait]
    impl CloudWatchClient for RecordingClient {
        async fn put_logs(
            &self,
            _dest: LogDestination,
            logs: Vec<LogEvent>,
        ) -> Result<(), crate::client::PutLogsError> {
            self.logs.lock().unwrap().extend(logs);
            Ok(())
        }
    }

    impl RecordingClient {
        fn exported_count(&self) -> usize {
            self.logs.lock().unwrap().len()
        }

        fn exported_messages(&self) -> Vec<String> {
            self.logs
                .lock()
                .unwrap()
                .iter()
                .map(|event| event.message.clone())
                .collect()
        }
    }

    async fn wait_for_exported_count(client: &RecordingClient, expected: usize) {
        timeout(Duration::from_secs(1), async {
            loop {
                if client.exported_count() >= expected {
                    break;
                }
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("timed out waiting for exported log events");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn drains_all_buffered_events_on_shutdown() {
        let client = RecordingClient::default();
        let exporter = BatchExporter::new(
            client.clone(),
            ExportConfig::default()
                .with_batch_size(10_000)
                .with_interval(Duration::from_secs(60))
                .with_log_group_name("group")
                .with_log_stream_name("stream"),
        );

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<ShutdownSignal>();
        let (shutdown_signal, _ack_rx) = ShutdownSignal::new();

        let total = 512;
        for idx in 0..total {
            tx.send(LogEvent {
                message: format!("event-{idx}"),
                timestamp: Utc::now(),
            })
            .unwrap();
        }
        drop(tx);
        shutdown_tx.send(shutdown_signal).unwrap();

        exporter.run(rx, shutdown_rx).await;

        assert_eq!(
            client.exported_count(),
            total,
            "all events queued before shutdown should be exported"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn exports_events_with_registry_on_guard_shutdown() {
        let client = RecordingClient::default();
        let (cw_layer, guard) = crate::layer()
            .with_code_location(false)
            .with_target(false)
            .with_client(
                client.clone(),
                ExportConfig::default()
                    .with_batch_size(1024)
                    .with_interval(Duration::from_secs(60))
                    .with_log_group_name("group")
                    .with_log_stream_name("stream"),
            );

        let subscriber = tracing_subscriber::registry().with(cw_layer);
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("integration-log-1");
            tracing::warn!("integration-log-2");
        });

        guard.shutdown().await;

        let messages = client.exported_messages();
        assert_eq!(messages.len(), 2);
        assert!(messages
            .iter()
            .any(|message| message.contains("integration-log-1")));
        assert!(messages
            .iter()
            .any(|message| message.contains("integration-log-2")));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn exports_when_batch_size_is_reached() {
        let client = RecordingClient::default();
        let (cw_layer, guard) = crate::layer()
            .with_code_location(false)
            .with_target(false)
            .with_client(
                client.clone(),
                ExportConfig::default()
                    .with_batch_size(2)
                    .with_interval(Duration::from_secs(60))
                    .with_log_group_name("group")
                    .with_log_stream_name("stream"),
            );

        let subscriber = tracing_subscriber::registry().with(cw_layer);
        // Let the exporter consume the initial immediate interval tick while the queue is empty.
        sleep(Duration::from_millis(20)).await;

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("batch-log-1");
            tracing::info!("batch-log-2");
        });

        wait_for_exported_count(&client, 2).await;
        guard.shutdown().await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn exports_without_shutdown_when_batch_not_full() {
        let client = RecordingClient::default();
        let (cw_layer, guard) = crate::layer()
            .with_code_location(false)
            .with_target(false)
            .with_client(
                client.clone(),
                ExportConfig::default()
                    .with_batch_size(1024)
                    .with_interval(Duration::from_millis(200))
                    .with_log_group_name("group")
                    .with_log_stream_name("stream"),
            );

        let subscriber = tracing_subscriber::registry().with(cw_layer);
        // Let the exporter consume the initial immediate interval tick while the queue is empty.
        sleep(Duration::from_millis(20)).await;

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("interval-log-1");
        });

        wait_for_exported_count(&client, 1).await;
        guard.shutdown().await;
    }
}
