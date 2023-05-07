use async_trait::async_trait;
use aws_sdk_cloudwatchlogs::{
    error::SdkError, operation::put_log_events::PutLogEventsError, types::InputLogEvent,
    Client as SdkClient,
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

        match self
            .put_log_events()
            .set_log_events(Some(log_events))
            .log_group_name(dest.log_group_name)
            .log_stream_name(dest.log_stream_name)
            .send()
            .await
        {
            Ok(output) => {
                if let Some(rejected_info) = output.rejected_log_events_info() {
                    eprintln!("[tracing-cloudwatch] Put logs rejected: {rejected_info:?}");
                }
                Ok(())
            }
            Err(SdkError::ServiceError(service_err)) => match service_err.into_err() {
                PutLogEventsError::ResourceNotFoundException(err) => {
                    Err(PutLogsError::LogDestinationNotFound {
                        message: err.message().unwrap_or_default().to_string(),
                    })
                }
                err => Err(anyhow::Error::from(err).into()),
            },
            Err(err) => Err(anyhow::Error::from(err).into()),
        }
    }
}

impl From<LogEvent> for InputLogEvent {
    fn from(value: LogEvent) -> Self {
        InputLogEvent::builder()
            .timestamp(value.timestamp.timestamp_millis())
            .message(value.message)
            .build()
    }
}
