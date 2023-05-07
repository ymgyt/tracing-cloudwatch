# tracing-cloudwatch

tracing-cloudwatch is a custom tracing-subscriber layer that sends your application's tracing events(logs) to AWS CloudWatch Logs.  

## Usage

### With Rusoto

feature `rusoto` required

```rust
use rusoto_core::Region;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    let cw_client = rusoto_logs::CloudWatchLogsClient::new(Region::ApNortheast1);

    tracing_subscriber::registry::Registry::default()
        .with(
            tracing_cloudwatch::layer().with_client(
                cw_client,
                tracing_cloudwatch::ExportConfig::default()
                    .with_batch_size(1)
                    .with_interval(Duration::from_secs(1))
                    .with_log_group_name("tracing-cloudwatch")
                    .with_log_stream_name("stream-1"),
            ),
        )
        .init();

    start().await;
}

#[tracing::instrument()]
async fn start() {
    tracing::info!("Starting...");
}
```

### With AWS SDK

feature `awssdk` required

```rust
#[tokio::main]
async fn main() {
    use tracing_subscriber::{filter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
    let config = aws_config::load_from_env().await;
    let cw_client = aws_sdk_cloudwatchlogs::Client::new(&config);

    tracing_subscriber::registry::Registry::default()
        .with(fmt::layer().with_ansi(true))
        .with(filter::LevelFilter::INFO)
        .with(
            tracing_cloudwatch::layer().with_client(
                cw_client,
                tracing_cloudwatch::ExportConfig::default()
                    .with_batch_size(1)
                    .with_interval(Duration::from_secs(1))
                    .with_log_group_name("tracing-cloudwatch")
                    .with_log_stream_name("stream-1"),
            ),
        )
        .init();

    start().await;
}

#[tracing::instrument()]
async fn start() {
    tracing::info!("Starting...");
}
```

## Required Permissions

Currently, following AWS IAM Permissions required

* `logs:PutLogEvents`

## License

This project is licensed under the [MIT license.](./LICENSE)