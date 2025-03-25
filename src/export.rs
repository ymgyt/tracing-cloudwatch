use std::fmt::Debug;
use std::num::NonZeroUsize;
use std::time::Duration;

use tokio::{sync::mpsc::UnboundedReceiver, time::interval};

use crate::{client::NoopClient, dispatch::LogEvent, CloudWatchClient};

/// Configurations to control the behavior of exporting logs to CloudWatch.
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// The number of logs to retain in the buffer within the interval period.
    batch_size: NonZeroUsize,
    /// The interval for putting logs.
    interval: Duration,
    /// To which logs sended.
    destination: LogDestination,
}

/// To which logs to sended.
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
    pub(crate) async fn run(self, mut rx: UnboundedReceiver<LogEvent>) {
        let BatchExporter {
            client,
            mut queue,
            config,
        } = self;

        let mut interval = interval(config.interval);

        loop {
            tokio::select! {
                 _ = interval.tick() => {
                    if queue.is_empty() {
                        continue;
                    }
                }
                event = rx.recv() => {
                    let Some(event) = event else {
                        break;
                    };

                    queue.push(event);
                    if queue.len() < <NonZeroUsize as Into<usize>>::into(config.batch_size) {
                        continue
                    }
                }
            }

            let logs: Vec<LogEvent> = Self::take_from_queue(&mut queue);

            if let Err(err) = client.put_logs(config.destination.clone(), logs).await {
                eprintln!(
                    "[tracing-cloudwatch] Unable to put logs to cloudwatch. Error: {err:?} {:?}",
                    config.destination
                );
            }
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
    use chrono::{DateTime, Utc};

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
}
