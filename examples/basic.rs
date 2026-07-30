//! Basic tracing_subscriber integration example.
//!
//! Logs one message at each log level.

use tracing_ecs_formatter::EcsFormatter;

fn main() {
    tracing_subscriber::fmt()
        .event_format(EcsFormatter::new("test-service", "1.0.0"))
        .with_max_level(tracing::Level::TRACE)
        .init();

    tracing::error!(
        error.type = "TestError",
        error.message = "something went wrong",
        error.stack_trace = "frame1\nframe2",
        custom_field = "custom_value",
        "error message"
    );
    tracing::warn!(custom_field = "custom_value", "warn message");
    tracing::info!(
        http.request.method = "GET",
        http.response.status_code = 200_u64,
        url.path = "/users",
        custom_field = "custom_value",
        "info message"
    );
    tracing::debug!(custom_field = "custom_value", "debug message");
    tracing::trace!(custom_field = "custom_value", "trace message");
}
