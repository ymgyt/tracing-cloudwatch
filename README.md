# tracing-cloudwatch

tracing-cloudwatch is a custom tracing-subscriber layer that sends your application's tracing events(logs) to AWS CloudWatch Logs.  

## Usage

### With Rusoto

feature `rusoto` required

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    let cw_client = rusoto_logs::CloudWatchLogsClient::new(rusoto_core::Region::ApNortheast1);

    tracing_subscriber::registry::Registry::default()
        .with(
            tracing_cloudwatch::layer().with_client(
                cw_client,
                tracing_cloudwatch::ExportConfig::default()
                    .with_batch_size(5)
                    .with_interval(Duration::from_secs(1))
                    .with_log_group_name("tracing-cloudwatch")
                    .with_log_stream_name("stream-1"),
            ),
        )
        .init();
}
```

### With AWS SDK

feature `awssdk` required

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    let config = aws_config::load_from_env().await;
    let cw_client = aws_sdk_cloudwatchlogs::Client::new(&config);

    tracing_subscriber::registry::Registry::default()
        .with(
            tracing_cloudwatch::layer().with_client(
                cw_client,
                tracing_cloudwatch::ExportConfig::default()
                    .with_batch_size(5)
                    .with_interval(Duration::from_secs(1))
                    .with_log_group_name("tracing-cloudwatch")
                    .with_log_stream_name("stream-1"),
            ),
        )
        .init();
}
```

## Required Permissions

Currently, following AWS IAM Permissions required

* `logs:PutLogEvents`

## License

This project is licensed under the [MIT license.](./LICENSE)