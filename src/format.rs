use crate::model::EcsLogRecord;
use crate::visitor::EcsLogRecordVisitor;
#[cfg(feature = "opentelemetry")]
use opentelemetry::trace::TraceContextExt;
use std::io;
use std::sync::Arc;
use tracing::{Event, Level, Subscriber};
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
/// [ECS 8.x specification](https://www.elastic.co/docs/reference/ecs) and the
/// [ECS logging spec](https://github.com/elastic/ecs-logging).
pub struct EcsFormatter {
    service_name: Arc<str>,
    service_version: Arc<str>,
    service_environment: Option<Arc<str>>,
    service_node_name: Option<Arc<str>>,
    event_dataset: Option<Arc<str>>,
    log_logger: Option<Arc<str>>,
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
        let service_name: Arc<str> = Arc::from(service_name.as_ref());
        let service_version: Arc<str> = Arc::from(service_version.as_ref());
        let event_dataset = Some(Arc::clone(&service_name));
        Self {
            service_name,
            service_version,
            service_environment: None,
            service_node_name: None,
            event_dataset,
            log_logger: None,
        }
    }

    /// Sets the optional `service.environment` field (e.g., `"prod"`, `"staging"`).
    pub fn with_service_environment(mut self, env: impl AsRef<str>) -> Self {
        self.service_environment = Some(Arc::from(env.as_ref()));
        self
    }

    /// Sets the optional `service.node.name` field (a unique node identifier).
    pub fn with_service_node_name(mut self, node: impl AsRef<str>) -> Self {
        self.service_node_name = Some(Arc::from(node.as_ref()));
        self
    }

    /// Overrides `event.dataset` (defaults to `service.name`, per the
    /// ecs-logging spec). Common values: `"my-service.access"`,
    /// `"my-service.audit"`.
    pub fn with_event_dataset(mut self, dataset: impl AsRef<str>) -> Self {
        self.event_dataset = Some(Arc::from(dataset.as_ref()));
        self
    }

    /// Sets the optional `log.logger` field — the application-chosen name of
    /// the logger instance emitting the event. When unset, the field is
    /// omitted from the output. `tracing` has no logger-instance concept, so
    /// there is no meaningful default to derive from event metadata.
    pub fn with_log_logger(mut self, name: impl AsRef<str>) -> Self {
        self.log_logger = Some(Arc::from(name.as_ref()));
        self
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
            self.log_logger.clone(),
            Arc::clone(&self.service_name),
            Arc::clone(&self.service_version),
            self.service_environment.clone(),
            self.service_node_name.clone(),
            self.event_dataset.clone(),
            metadata.target(),
        );

        #[cfg(feature = "opentelemetry")]
        {
            let span_ctx = opentelemetry::Context::current()
                .span()
                .span_context()
                .clone();
            if span_ctx.is_valid() {
                visitor
                    .trace_id(span_ctx.trace_id().to_string())
                    .span_id(span_ctx.span_id().to_string());
            }
        }

        let _ = ctx;

        if let (Some(file), Some(line)) = (metadata.file(), metadata.line()) {
            visitor.set_log_origin(file, line);
        }

        event.record(&mut visitor);

        let record: EcsLogRecord = visitor.into();
        if let Err(e) = serde_json::to_writer(FmtToIo(&mut writer), &record) {
            eprintln!("ECS serialization failed: {e}\n{record:?}");
            return Ok(());
        }
        writer.write_char('\n')
    }
}
