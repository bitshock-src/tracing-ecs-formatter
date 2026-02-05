use crate::model::EcsLogRecord;
use crate::visitor::EcsLogRecordVisitor;
#[cfg(feature = "opentelemetry")]
use opentelemetry::trace::TraceContextExt;
use std::io;
use std::sync::Arc;
use tracing::{Event, Level, Subscriber};
#[cfg(feature = "opentelemetry")]
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

struct FmtToIo<'a, 'b>(&'a mut Writer<'b>);

impl<'a, 'b> io::Write for FmtToIo<'a, 'b> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s =
            std::str::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.0.write_str(s).map_err(io::Error::other)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn level_to_str(level: &Level) -> &'static str {
    match *level {
        Level::ERROR => "ERROR",
        Level::WARN => "WARN",
        Level::INFO => "INFO",
        Level::DEBUG => "DEBUG",
        Level::TRACE => "TRACE",
    }
}

/// ECS (Elastic Common Schema) 8.11 JSON formatter for `tracing_subscriber`.
///
/// Implements [`FormatEvent`] to produce JSON log output conforming to the
/// [ECS 8.11 specification](https://www.elastic.co/guide/en/ecs/8.11/index.html).
pub struct EcsFormatter {
    service_name: Arc<str>,
    service_version: Arc<str>,
}

impl EcsFormatter {
    /// Creates a new ECS formatter with the given service metadata.
    ///
    /// # Arguments
    ///
    /// * `service_name` - Identifies the service producing logs (e.g., "api-gateway")
    /// * `service_version` - Service version (e.g., "1.2.3" or git commit hash)
    ///
    /// # Example
    ///
    /// ```
    /// use tracing_ecs_formatter::EcsFormatter;
    ///
    /// let formatter = EcsFormatter::new("my-service", env!("CARGO_PKG_VERSION"));
    /// ```
    pub fn new(service_name: impl AsRef<str>, service_version: impl AsRef<str>) -> Self {
        Self {
            service_name: Arc::from(service_name.as_ref()),
            service_version: Arc::from(service_version.as_ref()),
        }
    }
}

impl<S, N> FormatEvent<S, N> for EcsFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let metadata = event.metadata();

        let mut visitor = EcsLogRecordVisitor::new(
            chrono::Utc::now().to_rfc3339(),
            level_to_str(metadata.level()),
            metadata.target(),
            Arc::clone(&self.service_name),
            Arc::clone(&self.service_version),
        );

        #[cfg(feature = "opentelemetry")]
        if ctx.lookup_current().is_some() {
            let otel_ctx = tracing::Span::current().context();
            let otel_span = otel_ctx.span();
            let span_ctx = otel_span.span_context();

            if span_ctx.is_valid() {
                visitor
                    .trace_id(span_ctx.trace_id().to_string())
                    .span_id(span_ctx.span_id().to_string());
            }
        }

        #[cfg(not(feature = "opentelemetry"))]
        let _ = ctx;

        event.record(&mut visitor);

        let record: EcsLogRecord = visitor.into();
        if let Err(e) = serde_json::to_writer(FmtToIo(&mut writer), &record) {
            eprintln!("ECS serialization failed: {e}\n{record:?}");
            return Ok(());
        }
        writer.write_char('\n')
    }
}
