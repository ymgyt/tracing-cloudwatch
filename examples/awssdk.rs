#[cfg(feature = "awssdk")]
#[tokio::main]
async fn main() {
    use std::time::Duration;
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

    tokio::time::sleep(Duration::from_secs(10)).await;
}

#[cfg(not(feature = "awssdk"))]
fn main() {}

#[tracing::instrument()]
async fn start() {
    tracing::info!("Starting...");
}
