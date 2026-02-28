#[cfg(feature = "awssdk")]
#[tokio::main]
async fn main() {
    use aws_config::BehaviorVersion;
    use std::time::Duration;
    use tracing_subscriber::{filter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let cw_client = aws_sdk_cloudwatchlogs::Client::new(&config);

    let (cw_layer, _cw_guard) = tracing_cloudwatch::layer()
        .with_client(
            cw_client,
            tracing_cloudwatch::ExportConfig::default()
                .with_batch_size(1)
                .with_interval(Duration::from_secs(1))
                .with_log_group_name("tracing-cloudwatch")
                .with_log_stream_name("stream-1"),
        )
        .with_code_location(true)
        .with_target(false);

    tracing_subscriber::registry::Registry::default()
        .with(fmt::layer().with_ansi(true))
        .with(filter::LevelFilter::INFO)
        .with(cw_layer)
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
