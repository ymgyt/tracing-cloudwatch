# tracing-cloudwatch

tracing-cloudwatch is a custom tracing-subscriber layer that sends your application's tracing events(logs) to AWS CloudWatch Logs.

We have supported [rusoto](https://github.com/rusoto/rusoto) and the [AWS SDK](https://github.com/awslabs/aws-sdk-rust) as AWS clients.

## Usage

### With AWS SDK

feature `awssdk` required

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let cw_client = aws_sdk_cloudwatchlogs::Client::new(&config);

    tracing_subscriber::registry::Registry::default()
        .with(
            tracing_cloudwatch::layer().with_client(
                cw_client,
                tracing_cloudwatch::ExportConfig::default()
                    .with_batch_size(5)
                    .with_interval(std::time::Duration::from_secs(1))
                    .with_log_group_name("tracing-cloudwatch")
                    .with_log_stream_name("stream-1"),
            )
            .with_code_location(true)
            .with_target(false),
        )
        .init();
}
```

#### Chronological order

When aggregating logs from multiple places (or integrations such as [tracing-gstreamer](https://crates.io/crates/tracing-gstreamer)), messages can become unordered. This causes a `InvalidParameterException: Log events in a single PutLogEvents request must be in chronological order.` error from the CloudWatch client. To mediate this, you may enable the `ordered_logs` feature. Take into consideration that this can possibly increase processing time significantly depending on the number of events in the batch. Your milage may vary!

There is some additional context in https://github.com/ymgyt/tracing-cloudwatch/issues/40

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
                    .with_interval(std::time::Duration::from_secs(1))
                    .with_log_group_name("tracing-cloudwatch")
                    .with_log_stream_name("stream-1"),
            )
            .with_code_location(true)
            .with_target(false),
        )
        .init();
}
```

### Using pre-configured `tracing_subsriber::fmt::Layer`

You can specify a pre-configured [`fmt::Layer`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/struct.Layer.html) to control the log format.
For example, the following example outputs the logs in JSON format.

```rust
tracing_subscriber::registry::Registry::default()
    .with(tracing_cloudwatch::layer()
        .with_fmt_layer(
            tracing_subscriber::fmt::layer()
                .json()
                .without_time()
        )
    )
    .init();
```

## Required Permissions

Currently, following AWS IAM Permissions required

- `logs:PutLogEvents`

## CloudWatch Log Groups and Streams

This crate does not create a log group and log stream, so if the specified log group and log stream does not exist, it will raise an error.

## Retry and Timeout

We haven't implemented any custom retry logic or timeout settings within the crate. We assume that these configurations are handled through the SDK Client.  
For instance, in the AWS SDK, you can set up these configurations using [`timeout_config`](https://docs.rs/aws-sdk-cloudwatchlogs/0.28.0/aws_sdk_cloudwatchlogs/config/struct.Builder.html#method.timeout_config) and [`retry_config`](https://docs.rs/aws-sdk-cloudwatchlogs/0.28.0/aws_sdk_cloudwatchlogs/config/struct.Builder.html#method.retry_config)

## License

This project is licensed under the [MIT license.](./LICENSE)
