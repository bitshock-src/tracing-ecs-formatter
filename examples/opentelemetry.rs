//! OpenTelemetry integration example.
//!
//! Logs one message at each log level within an OpenTelemetry span.

use opentelemetry::trace::TracerProvider;
use tracing_ecs_formatter::EcsFormatter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn main() {
    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder().build();
    let tracer = provider.tracer("test-tracer");

    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::TRACE)
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(
            tracing_subscriber::fmt::layer()
                .event_format(EcsFormatter::new("test-service", "1.0.0")),
        )
        .init();

    let span = tracing::info_span!("test_span");
    let _guard = span.enter();

    tracing::error!(
        error.type = "TestError",
        error.message = "something went wrong",
        error.stack_trace = "frame1\nframe2",
        custom_field = "custom_value",
        "error message"
    );
    tracing::warn!(custom_field = "custom_value", "warn message");
    tracing::info!(custom_field = "custom_value", "info message");
    tracing::debug!(custom_field = "custom_value", "debug message");
    tracing::trace!(custom_field = "custom_value", "trace message");
}
