use crate::model::{EcsError, EcsLogRecord};
use std::collections::HashMap;
use std::sync::Arc;

const ECS_VERSION: &str = "8.11";

#[derive(Debug, Default, PartialEq)]
pub struct VisitedFields {
    pub message: Option<String>,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
    pub error_stack_trace: Option<String>,
    pub labels: HashMap<String, String>,
}

impl VisitedFields {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_value(&mut self, name: &str, value: String) {
        match name {
            "message" => self.message = Some(value),
            "error.type" => self.error_type = Some(value),
            "error.message" => self.error_message = Some(value),
            "error.stack_trace" => self.error_stack_trace = Some(value),
            _ => {
                let mut key = String::with_capacity(7 + name.len());
                key.push_str("labels.");
                key.push_str(name);
                self.labels.insert(key, value);
            }
        }
    }
}

pub struct EcsLogRecordVisitor {
    timestamp: String,
    log_level: &'static str,
    log_logger: String,
    service_name: Arc<str>,
    service_version: Arc<str>,
    trace_id: Option<String>,
    span_id: Option<String>,
    visited: VisitedFields,
}

impl EcsLogRecordVisitor {
    pub fn new(
        timestamp: impl Into<String>,
        log_level: &'static str,
        log_logger: impl Into<String>,
        service_name: Arc<str>,
        service_version: Arc<str>,
    ) -> Self {
        Self::with_visited(
            timestamp,
            log_level,
            log_logger,
            service_name,
            service_version,
            VisitedFields::new(),
        )
    }

    pub fn with_visited(
        timestamp: impl Into<String>,
        log_level: &'static str,
        log_logger: impl Into<String>,
        service_name: Arc<str>,
        service_version: Arc<str>,
        visited: VisitedFields,
    ) -> Self {
        Self {
            timestamp: timestamp.into(),
            log_level,
            log_logger: log_logger.into(),
            service_name,
            service_version,
            trace_id: None,
            span_id: None,
            visited,
        }
    }

    #[cfg(feature = "opentelemetry")]
    pub fn trace_id(&mut self, value: impl Into<String>) -> &mut Self {
        self.trace_id = Some(value.into());
        self
    }

    #[cfg(feature = "opentelemetry")]
    pub fn span_id(&mut self, value: impl Into<String>) -> &mut Self {
        self.span_id = Some(value.into());
        self
    }
}

impl From<EcsLogRecordVisitor> for EcsLogRecord {
    fn from(mut visitor: EcsLogRecordVisitor) -> Self {
        let has_error = visitor.visited.error_message.is_some()
            || visitor.visited.error_type.is_some()
            || visitor.visited.error_stack_trace.is_some();

        let message = match visitor.visited.message.take() {
            Some(m) if !m.is_empty() => m,
            _ => visitor.visited.error_message.clone().unwrap_or_default(),
        };

        let error = if has_error {
            Some(EcsError {
                error_type: visitor.visited.error_type,
                error_message: visitor.visited.error_message.unwrap_or_default(),
                stack_trace: visitor.visited.error_stack_trace,
            })
        } else {
            None
        };

        EcsLogRecord {
            timestamp: visitor.timestamp,
            log_level: visitor.log_level,
            message,
            ecs_version: ECS_VERSION,
            trace_id: visitor.trace_id,
            span_id: visitor.span_id,
            service_name: visitor.service_name,
            service_version: visitor.service_version,
            log_logger: visitor.log_logger,
            error,
            labels: visitor.visited.labels,
        }
    }
}

impl tracing::field::Visit for EcsLogRecordVisitor {
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.visited.record_value(field.name(), value.to_string());
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.visited.record_value(field.name(), value.to_string());
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.visited.record_value(field.name(), value.to_string());
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.visited.record_value(field.name(), value.to_string());
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.visited
            .record_value(field.name(), format!("{:?}", value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tracing::field::{Field, Visit};
    use tracing::{Callsite, callsite};

    fn make_field(name: &'static str) -> Field {
        struct TestCallsite;

        impl Callsite for TestCallsite {
            fn set_interest(&self, _: tracing::subscriber::Interest) {}
            fn metadata(&self) -> &tracing::Metadata<'_> {
                static FIELD_NAMES: &[&str] = &[
                    "message",
                    "error.type",
                    "error.message",
                    "error.stack_trace",
                    "custom_label",
                    "count",
                    "enabled",
                ];

                static META: tracing::Metadata<'static> = tracing::Metadata::new(
                    "test",
                    "test",
                    tracing::Level::INFO,
                    None,
                    None,
                    None,
                    tracing::field::FieldSet::new(FIELD_NAMES, callsite::Identifier(&TestCallsite)),
                    tracing::metadata::Kind::EVENT,
                );
                &META
            }
        }

        TestCallsite
            .metadata()
            .fields()
            .field(name)
            .unwrap_or_else(|| panic!("unknown field: {}", name))
    }

    #[test]
    fn test_visited_fields_record_value() {
        let mut visited = VisitedFields::new();

        visited.record_value("message", "test message".to_string());
        assert_eq!(visited.message, Some("test message".to_string()));

        visited.record_value("error.type", "TestError".to_string());
        assert_eq!(visited.error_type, Some("TestError".to_string()));

        visited.record_value("error.message", "error occurred".to_string());
        assert_eq!(visited.error_message, Some("error occurred".to_string()));

        visited.record_value("error.stack_trace", "stack trace here".to_string());
        assert_eq!(
            visited.error_stack_trace,
            Some("stack trace here".to_string())
        );

        visited.record_value("custom_field", "custom_value".to_string());
        assert_eq!(
            visited.labels.get("labels.custom_field"),
            Some(&"custom_value".to_string())
        );
    }

    #[test]
    fn test_visit_trait_methods() {
        let mut visitor = EcsLogRecordVisitor::new(
            "2024-01-15T10:30:00Z",
            "INFO",
            "test",
            Arc::from("svc"),
            Arc::from("1.0"),
        );

        visitor.record_str(&make_field("message"), "str value");
        assert_eq!(visitor.visited.message, Some("str value".to_string()));

        visitor.record_i64(&make_field("error.type"), -42);
        assert_eq!(visitor.visited.error_type, Some("-42".to_string()));

        visitor.record_u64(&make_field("error.message"), 123);
        assert_eq!(visitor.visited.error_message, Some("123".to_string()));

        visitor.record_bool(&make_field("error.stack_trace"), true);
        assert_eq!(visitor.visited.error_stack_trace, Some("true".to_string()));

        visitor.record_debug(&make_field("custom_label"), &vec![1, 2, 3]);
        assert_eq!(
            visitor.visited.labels.get("labels.custom_label"),
            Some(&"[1, 2, 3]".to_string())
        );
    }

    #[test]
    fn test_into_ecs_log_record() {
        let expected = EcsLogRecord {
            timestamp: "2024-01-15T10:30:00Z".to_string(),
            log_level: "ERROR",
            message: "Something went wrong".to_string(),
            ecs_version: "8.11",
            trace_id: None,
            span_id: None,
            service_name: Arc::from("test-service"),
            service_version: Arc::from("1.2.3"),
            log_logger: "my_app::module".to_string(),
            error: Some(EcsError {
                error_type: Some("ValidationError".to_string()),
                error_message: "Invalid input".to_string(),
                stack_trace: Some("at line 42".to_string()),
            }),
            labels: {
                let mut labels = HashMap::new();
                labels.insert("labels.custom".to_string(), "value".to_string());
                labels
            },
        };

        let visited = VisitedFields {
            message: Some(expected.message.clone()),
            error_type: expected.error.as_ref().unwrap().error_type.clone(),
            error_message: Some(expected.error.as_ref().unwrap().error_message.clone()),
            error_stack_trace: expected.error.as_ref().unwrap().stack_trace.clone(),
            labels: expected.labels.clone(),
        };

        let visitor = EcsLogRecordVisitor::with_visited(
            expected.timestamp.clone(),
            expected.log_level,
            expected.log_logger.clone(),
            Arc::clone(&expected.service_name),
            Arc::clone(&expected.service_version),
            visited,
        );

        let actual: EcsLogRecord = visitor.into();

        assert_eq!(actual, expected);
    }
}
