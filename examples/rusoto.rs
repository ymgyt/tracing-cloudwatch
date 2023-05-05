#[cfg(feature = "rusoto")]
#[tokio::main]
async fn main() {
    use rusoto_core::Region;
    use std::time::Duration;
    use tracing::info;
    use tracing_subscriber::{filter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
    let cw_client = rusoto_logs::CloudWatchLogsClient::new(Region::ApNortheast1);

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

    tokio::time::sleep(Duration::from_secs(5)).await;
}

#[cfg(not(feature = "rusoto"))]
fn main() {}

#[tracing::instrument()]
async fn start() {
    tracing::info!("Starting...");
}
