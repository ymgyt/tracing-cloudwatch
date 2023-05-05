use async_trait::async_trait;
use rusoto_core::RusotoError;
use rusoto_logs::{
    CloudWatchLogs, CloudWatchLogsClient as SdkClient, InputLogEvent, PutLogEventsError,
    PutLogEventsRequest,
};

use crate::{
    client::{CloudWatchClient, LogDestination, PutLogsError},
    dispatch::LogEvent,
};

#[async_trait]
impl CloudWatchClient for SdkClient {
    async fn put_logs(
        &self,
        dest: LogDestination,
        logs: Vec<LogEvent>,
    ) -> Result<(), PutLogsError> {
        let log_events = logs.into_iter().map(From::from).collect();

        let input = PutLogEventsRequest {
            log_events,
            log_group_name: dest.log_group_name,
            log_stream_name: dest.log_stream_name,
            sequence_token: None,
        };

        // TODO: retry
        // Is the next sequence token no longer used?
        // https://docs.aws.amazon.com/AmazonCloudWatchLogs/latest/APIReference/API_PutLogEvents.html
        match self.put_log_events(input).await {
            Ok(response) => {
                if let Some(rejected_info) = response.rejected_log_events_info {
                    eprintln!("[tracing-cloudwatch] Put logs rejected: {rejected_info:?}")
                }
                Ok(())
            }
            Err(RusotoError::Service(PutLogEventsError::ResourceNotFound(message))) => {
                Err(PutLogsError::LogDestinationNotFound { message })
            }
            Err(err) => Err(anyhow::Error::from(err).into()),
        }
    }
}

impl From<LogEvent> for InputLogEvent {
    fn from(value: LogEvent) -> Self {
        Self {
            message: value.message,
            timestamp: value.timestamp.timestamp_millis(),
        }
    }
}
