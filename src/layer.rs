use std::sync::Arc;

use tracing_core::{span, Event, Subscriber};
use tracing_subscriber::{
    fmt::{self, format, MakeWriter},
    layer::Context,
    registry::LookupSpan,
    Layer,
};

use crate::{
    client::CloudWatchClient,
    dispatch::{CloudWatchDispatcher, Dispatcher, NoopDispatcher},
    export::ExportConfig,
};

/// An AWS Cloudwatch propagation layer.
pub struct CloudWatchLayer<S, D> {
    fmt_layer: fmt::Layer<S, format::DefaultFields, format::Format<format::Full, ()>, Arc<D>>,
}

/// Construct [CloudWatchLayer] to compose with tracing subscriber.
pub fn layer<S>() -> CloudWatchLayer<S, NoopDispatcher>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    CloudWatchLayer::default()
}

impl<S> Default for CloudWatchLayer<S, NoopDispatcher>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn default() -> Self {
        CloudWatchLayer::new(Arc::new(NoopDispatcher::new()))
    }
}

impl<S, D> CloudWatchLayer<S, D>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    D: Dispatcher + 'static,
    Arc<D>: for<'writer> MakeWriter<'writer>,
{
    pub fn new(dispatcher: Arc<D>) -> Self {
        Self {
            fmt_layer: fmt::Layer::default()
                .without_time()
                .with_writer(dispatcher)
                .with_ansi(false)
                .with_level(true)
                .with_line_number(true)
                .with_file(true)
                .with_target(false),
        }
    }

    /// Set client.
    pub fn with_client<Client>(
        self,
        client: Client,
        export_config: ExportConfig,
    ) -> CloudWatchLayer<S, CloudWatchDispatcher>
    where
        Client: CloudWatchClient + Send + Sync + 'static,
    {
        CloudWatchLayer {
            fmt_layer: self
                .fmt_layer
                .with_writer(Arc::new(CloudWatchDispatcher::new(client, export_config))),
        }
    }

    /// Configure to display line number and filename.
    /// Default true
    pub fn with_code_location(self, display: bool) -> Self {
        Self {
            fmt_layer: self.fmt_layer.with_line_number(display).with_file(display),
        }
    }

    /// Configure to display target module.
    /// Default false.
    pub fn with_target(self, display: bool) -> Self {
        Self {
            fmt_layer: self.fmt_layer.with_target(display),
        }
    }
}

impl<S, D> Layer<S> for CloudWatchLayer<S, D>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    D: Dispatcher + 'static,
    Arc<D>: for<'writer> MakeWriter<'writer>,
{
    fn on_enter(&self, id: &span::Id, ctx: Context<'_, S>) {
        self.fmt_layer.on_enter(id, ctx)
    }
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        self.fmt_layer.on_event(event, ctx)
    }

    fn on_register_dispatch(&self, collector: &tracing::Dispatch) {
        self.fmt_layer.on_register_dispatch(collector)
    }

    fn on_layer(&mut self, subscriber: &mut S) {
        let _ = subscriber;
    }

    fn enabled(&self, metadata: &tracing::Metadata<'_>, ctx: Context<'_, S>) -> bool {
        self.fmt_layer.enabled(metadata, ctx)
    }

    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        self.fmt_layer.on_new_span(attrs, id, ctx)
    }

    fn on_record(&self, id: &span::Id, values: &span::Record<'_>, ctx: Context<'_, S>) {
        self.fmt_layer.on_record(id, values, ctx)
    }

    fn on_follows_from(&self, span: &span::Id, follows: &span::Id, ctx: Context<'_, S>) {
        self.fmt_layer.on_follows_from(span, follows, ctx)
    }

    fn event_enabled(&self, event: &Event<'_>, ctx: Context<'_, S>) -> bool {
        self.fmt_layer.event_enabled(event, ctx)
    }

    fn on_exit(&self, id: &span::Id, ctx: Context<'_, S>) {
        self.fmt_layer.on_exit(id, ctx)
    }

    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        self.fmt_layer.on_close(id, ctx)
    }

    fn on_id_change(&self, old: &span::Id, new: &span::Id, ctx: Context<'_, S>) {
        self.fmt_layer.on_id_change(old, new, ctx)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use chrono::{DateTime, TimeZone, Utc};
    use tracing_subscriber::layer::SubscriberExt;

    use crate::dispatch::LogEvent;

    use super::*;

    struct TestDispatcher {
        events: Mutex<Vec<LogEvent>>,
    }

    impl TestDispatcher {
        fn new() -> Self {
            Self {
                events: Mutex::new(Vec::new()),
            }
        }
    }

    impl Dispatcher for TestDispatcher {
        fn dispatch(&self, input: crate::dispatch::LogEvent) {
            self.events.lock().unwrap().push(input)
        }
    }

    impl std::io::Write for &TestDispatcher {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let timestamp: DateTime<Utc> = Utc.timestamp_opt(1_5000_000_000, 0).unwrap();

            let message = String::from_utf8_lossy(buf).to_string();

            self.events
                .lock()
                .unwrap()
                .push(LogEvent { timestamp, message });

            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn format() {
        let dispatcher = Arc::new(TestDispatcher::new());
        let subscriber = tracing_subscriber::registry().with(
            CloudWatchLayer::new(dispatcher.clone())
                .with_code_location(false)
                .with_target(false),
        );

        tracing::subscriber::with_default(subscriber, || {
            tracing::info_span!("span-1", xxx = "yyy").in_scope(|| {
                tracing::debug_span!("span-2", key = "value").in_scope(|| {
                    tracing::info!("Hello!");
                })
            });

            tracing::error!("Error");
        });

        let dispatched = dispatcher.events.lock().unwrap().remove(0);
        assert_eq!(
            dispatched.message,
            " INFO span-1{xxx=\"yyy\"}:span-2{key=\"value\"}: Hello!\n"
        );

        let dispatched = dispatcher.events.lock().unwrap().remove(0);
        assert_eq!(dispatched.message, "ERROR Error\n");
    }
}
