use crate::model::EcsLogRecord;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

const ECS_VERSION: &str = "8.11";

const ECS_NAMESPACES: &[&str] = &[
    "agent",
    "as",
    "client",
    "cloud",
    "code_signature",
    "container",
    "custom",
    "data_stream",
    "destination",
    "device",
    "dll",
    "dns",
    "ecs",
    "elf",
    "email",
    "entity",
    "entity_reference",
    "error",
    "event",
    "faas",
    "file",
    "gen_ai",
    "geo",
    "group",
    "hash",
    "host",
    "http",
    "interface",
    "log",
    "macho",
    "network",
    "observer",
    "orchestrator",
    "organization",
    "os",
    "package",
    "pe",
    "process",
    "registry",
    "related",
    "risk",
    "rule",
    "server",
    "service",
    "source",
    "threat",
    "tls",
    "tracing",
    "url",
    "user",
    "user_agent",
    "vlan",
    "volume",
    "vulnerability",
    "x509",
];

fn is_ecs_namespace_field(name: &str) -> bool {
    match name.split_once('.') {
        Some((ns, _)) => ECS_NAMESPACES.binary_search(&ns).is_ok(),
        None => false,
    }
}

const ECS_ARRAY_FIELDS: &[&str] = &["event.category", "event.type"];

fn is_ecs_array_field(name: &str) -> bool {
    ECS_ARRAY_FIELDS.binary_search(&name).is_ok()
}

fn coerce_to_array(value: Value) -> Value {
    match value {
        Value::Array(_) => value,
        Value::String(ref s) if s.trim_start().starts_with('[') => {
            match serde_json::from_str::<Value>(s) {
                Ok(Value::Array(arr)) => Value::Array(arr),
                _ => Value::Array(vec![value]),
            }
        }
        _ => Value::Array(vec![value]),
    }
}

fn sanitize_label_key(name: &str) -> String {
    let mut out = String::with_capacity("labels.".len() + name.len());
    out.push_str("labels.");
    for c in name.chars() {
        match c {
            '.' | '*' | '\\' => out.push('_'),
            _ => out.push(c),
        }
    }
    out
}

fn value_to_label_string(value: Value) -> String {
    match value {
        Value::String(s) => s,
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct VisitedFields {
    pub message: Option<String>,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
    pub error_stack_trace: Option<String>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub ecs_fields: HashMap<String, Value>,
    pub labels: HashMap<String, String>,
}

impl VisitedFields {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, name: &str, value: Value) {
        match name {
            "message" => {
                self.message = Some(value_to_label_string(value));
                return;
            }
            "error.type" => {
                self.error_type = Some(value_to_label_string(value));
                return;
            }
            "error.message" => {
                self.error_message = Some(value_to_label_string(value));
                return;
            }
            "error.stack_trace" => {
                self.error_stack_trace = Some(value_to_label_string(value));
                return;
            }
            "trace.id" => {
                self.trace_id = Some(value_to_label_string(value));
                return;
            }
            "span.id" => {
                self.span_id = Some(value_to_label_string(value));
                return;
            }
            _ => {}
        }
        if is_ecs_namespace_field(name) {
            let stored = if is_ecs_array_field(name) {
                coerce_to_array(value)
            } else {
                value
            };
            self.ecs_fields.insert(name.to_string(), stored);
        } else {
            self.labels
                .insert(sanitize_label_key(name), value_to_label_string(value));
        }
    }
}

pub struct EcsLogRecordVisitor {
    timestamp: String,
    log_level: &'static str,
    log_logger: Option<Arc<str>>,
    service_name: Arc<str>,
    service_version: Arc<str>,
    service_environment: Option<Arc<str>>,
    service_node_name: Option<Arc<str>>,
    event_dataset: Option<Arc<str>>,
    event_module: String,
    trace_id: Option<String>,
    span_id: Option<String>,
    visited: VisitedFields,
}

impl EcsLogRecordVisitor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        timestamp: impl Into<String>,
        log_level: &'static str,
        log_logger: Option<Arc<str>>,
        service_name: Arc<str>,
        service_version: Arc<str>,
        service_environment: Option<Arc<str>>,
        service_node_name: Option<Arc<str>>,
        event_dataset: Option<Arc<str>>,
        event_module: impl Into<String>,
    ) -> Self {
        Self::with_visited(
            timestamp,
            log_level,
            log_logger,
            service_name,
            service_version,
            service_environment,
            service_node_name,
            event_dataset,
            event_module,
            VisitedFields::new(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_visited(
        timestamp: impl Into<String>,
        log_level: &'static str,
        log_logger: Option<Arc<str>>,
        service_name: Arc<str>,
        service_version: Arc<str>,
        service_environment: Option<Arc<str>>,
        service_node_name: Option<Arc<str>>,
        event_dataset: Option<Arc<str>>,
        event_module: impl Into<String>,
        visited: VisitedFields,
    ) -> Self {
        Self {
            timestamp: timestamp.into(),
            log_level,
            log_logger,
            service_name,
            service_version,
            service_environment,
            service_node_name,
            event_dataset,
            event_module: event_module.into(),
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

    pub(crate) fn set_log_origin(&mut self, file: &str, line: u32) {
        self.visited
            .ecs_fields
            .insert("log.origin.file.name".to_string(), Value::from(file));
        self.visited
            .ecs_fields
            .insert("log.origin.file.line".to_string(), Value::from(line));
    }
}

impl From<EcsLogRecordVisitor> for EcsLogRecord {
    fn from(visitor: EcsLogRecordVisitor) -> Self {
        EcsLogRecord {
            timestamp: visitor.timestamp,
            log_level: visitor.log_level,
            message: visitor.visited.message,
            ecs_version: ECS_VERSION,
            trace_id: visitor.visited.trace_id.or(visitor.trace_id),
            span_id: visitor.visited.span_id.or(visitor.span_id),
            service_name: visitor.service_name,
            service_version: visitor.service_version,
            service_environment: visitor.service_environment,
            service_node_name: visitor.service_node_name,
            event_dataset: visitor.event_dataset,
            event_module: visitor.event_module,
            log_logger: visitor.log_logger,
            error_type: visitor.visited.error_type,
            error_message: visitor.visited.error_message,
            error_stack_trace: visitor.visited.error_stack_trace,
            ecs_fields: visitor.visited.ecs_fields,
            labels: visitor.visited.labels,
        }
    }
}

impl tracing::field::Visit for EcsLogRecordVisitor {
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.visited.record(field.name(), Value::from(value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.visited.record(field.name(), Value::from(value));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        let v = serde_json::Number::from_f64(value)
            .map(Value::Number)
            .unwrap_or_else(|| Value::from(value.to_string()));
        self.visited.record(field.name(), v);
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.visited.record(field.name(), Value::from(value));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.visited.record(field.name(), Value::from(value));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.visited
            .record(field.name(), Value::from(format!("{:?}", value)));
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
                    "http.request.method",
                    "http.response.status_code",
                    "user_id",
                    "htp.request.method",
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
    fn ecs_namespace_list_is_sorted() {
        let mut sorted = ECS_NAMESPACES.to_vec();
        sorted.sort_unstable();
        assert_eq!(ECS_NAMESPACES, sorted.as_slice());
    }

    #[test]
    fn ecs_array_field_list_is_sorted() {
        let mut sorted = ECS_ARRAY_FIELDS.to_vec();
        sorted.sort_unstable();
        assert_eq!(ECS_ARRAY_FIELDS, sorted.as_slice());
    }

    #[test]
    fn is_ecs_array_field_recognises_normalize_array_fields() {
        assert!(is_ecs_array_field("event.category"));
        assert!(is_ecs_array_field("event.type"));
        assert!(!is_ecs_array_field("event.kind"));
        assert!(!is_ecs_array_field("event.outcome"));
        assert!(!is_ecs_array_field("http.request.method"));
    }

    #[test]
    fn coerce_to_array_wraps_scalar_string() {
        assert_eq!(
            coerce_to_array(Value::from("access")),
            Value::Array(vec![Value::from("access")]),
        );
    }

    #[test]
    fn coerce_to_array_passes_through_actual_array() {
        let arr = Value::Array(vec![Value::from("a"), Value::from("b")]);
        assert_eq!(coerce_to_array(arr.clone()), arr);
    }

    #[test]
    fn coerce_to_array_parses_json_array_literal_string() {
        assert_eq!(
            coerce_to_array(Value::from(r#"["access","error"]"#)),
            Value::Array(vec![Value::from("access"), Value::from("error")]),
        );
    }

    #[test]
    fn coerce_to_array_falls_back_to_scalar_wrap_on_malformed_json() {
        assert_eq!(
            coerce_to_array(Value::from("[unterminated")),
            Value::Array(vec![Value::from("[unterminated")]),
        );
    }

    #[test]
    fn coerce_to_array_ignores_non_bracket_strings() {
        assert_eq!(
            coerce_to_array(Value::from("web,api")),
            Value::Array(vec![Value::from("web,api")]),
        );
    }

    #[test]
    fn coerce_to_array_wraps_number_and_bool() {
        assert_eq!(
            coerce_to_array(Value::from(42_i64)),
            Value::Array(vec![Value::from(42_i64)]),
        );
        assert_eq!(
            coerce_to_array(Value::from(true)),
            Value::Array(vec![Value::from(true)]),
        );
    }

    #[test]
    fn record_coerces_scalar_for_array_field() {
        let mut visited = VisitedFields::new();
        visited.record("event.type", Value::from("access"));
        assert_eq!(
            visited.ecs_fields.get("event.type"),
            Some(&Value::Array(vec![Value::from("access")])),
        );
    }

    #[test]
    fn record_preserves_multi_element_array_value_for_array_field() {
        let mut visited = VisitedFields::new();
        let arr = Value::Array(vec![Value::from("web"), Value::from("api")]);
        visited.record("event.category", arr.clone());
        assert_eq!(visited.ecs_fields.get("event.category"), Some(&arr));
    }

    #[test]
    fn record_does_not_coerce_scalar_for_non_array_field() {
        let mut visited = VisitedFields::new();
        visited.record("event.kind", Value::from("event"));
        assert_eq!(
            visited.ecs_fields.get("event.kind"),
            Some(&Value::from("event")),
        );
    }

    #[test]
    fn is_ecs_namespace_field_recognises_allowlisted_prefixes() {
        assert!(is_ecs_namespace_field("http.request.method"));
        assert!(is_ecs_namespace_field("client.ip"));
        assert!(is_ecs_namespace_field("event.duration"));
        assert!(is_ecs_namespace_field("custom.anything"));
    }

    #[test]
    fn sanitize_label_key_replaces_specials() {
        assert_eq!(sanitize_label_key("user.id"), "labels.user_id");
        assert_eq!(sanitize_label_key("weird*name"), "labels.weird_name");
        assert_eq!(
            sanitize_label_key("path\\to\\thing"),
            "labels.path_to_thing"
        );
        assert_eq!(
            sanitize_label_key("weird.name*with\\slash"),
            "labels.weird_name_with_slash"
        );
    }

    #[test]
    fn record_special_cases_populate_typed_slots() {
        let mut visited = VisitedFields::new();

        visited.record("message", Value::from("test message"));
        assert_eq!(visited.message.as_deref(), Some("test message"));

        visited.record("error.type", Value::from("TestError"));
        assert_eq!(visited.error_type.as_deref(), Some("TestError"));

        visited.record("error.message", Value::from("error occurred"));
        assert_eq!(visited.error_message.as_deref(), Some("error occurred"));

        visited.record("error.stack_trace", Value::from("frame1\nframe2"));
        assert_eq!(visited.error_stack_trace.as_deref(), Some("frame1\nframe2"));
    }

    #[test]
    fn record_routes_ecs_namespaces_to_typed_fields() {
        let mut visited = VisitedFields::new();

        visited.record("http.request.method", Value::from("GET"));
        assert_eq!(
            visited.ecs_fields.get("http.request.method"),
            Some(&Value::from("GET"))
        );

        visited.record("http.response.status_code", Value::from(201_u16));
        assert_eq!(
            visited.ecs_fields.get("http.response.status_code"),
            Some(&Value::from(201_u16))
        );
    }

    #[test]
    fn record_routes_unknown_to_sanitized_labels() {
        let mut visited = VisitedFields::new();

        visited.record("user_id", Value::from(42_i64));
        assert_eq!(
            visited.labels.get("labels.user_id").map(String::as_str),
            Some("42")
        );

        visited.record("htp.request.method", Value::from("GET"));
        assert_eq!(
            visited
                .labels
                .get("labels.htp_request_method")
                .map(String::as_str),
            Some("GET")
        );
    }

    #[test]
    fn visit_trait_methods_preserve_types_in_ecs_fields() {
        let mut visitor = EcsLogRecordVisitor::new(
            "2024-01-15T10:30:00Z",
            "INFO",
            None,
            Arc::from("svc"),
            Arc::from("1.0"),
            None,
            None,
            None,
            "test",
        );

        visitor.record_str(&make_field("http.request.method"), "GET");
        visitor.record_u64(&make_field("http.response.status_code"), 201);
        visitor.record_i64(&make_field("user_id"), 42);
        visitor.record_bool(&make_field("enabled"), true);
        visitor.record_debug(&make_field("custom_label"), &vec![1, 2, 3]);

        assert_eq!(
            visitor.visited.ecs_fields.get("http.request.method"),
            Some(&Value::from("GET"))
        );
        assert_eq!(
            visitor.visited.ecs_fields.get("http.response.status_code"),
            Some(&Value::from(201_u64))
        );
        assert_eq!(
            visitor
                .visited
                .labels
                .get("labels.user_id")
                .map(String::as_str),
            Some("42")
        );
        assert_eq!(
            visitor
                .visited
                .labels
                .get("labels.enabled")
                .map(String::as_str),
            Some("true")
        );
        assert_eq!(
            visitor
                .visited
                .labels
                .get("labels.custom_label")
                .map(String::as_str),
            Some("[1, 2, 3]")
        );
    }

    #[test]
    fn set_log_origin_inserts_ecs_fields() {
        let mut visitor = EcsLogRecordVisitor::new(
            "2024-01-15T10:30:00Z",
            "INFO",
            None,
            Arc::from("svc"),
            Arc::from("1.0"),
            None,
            None,
            None,
            "test",
        );

        visitor.set_log_origin("src/foo.rs", 42);

        assert_eq!(
            visitor.visited.ecs_fields.get("log.origin.file.name"),
            Some(&Value::from("src/foo.rs"))
        );
        assert_eq!(
            visitor.visited.ecs_fields.get("log.origin.file.line"),
            Some(&Value::from(42_u32))
        );
    }

    #[test]
    fn recorded_origin_fields_overwrite_metadata_seed() {
        let mut visitor = EcsLogRecordVisitor::new(
            "2024-01-15T10:30:00Z",
            "INFO",
            None,
            Arc::from("svc"),
            Arc::from("1.0"),
            None,
            None,
            None,
            "test",
        );

        visitor.set_log_origin("src/foo.rs", 42);
        visitor
            .visited
            .record("log.origin.file.name", Value::from("user.rs"));
        visitor
            .visited
            .record("log.origin.file.line", Value::from(999_u64));

        assert_eq!(
            visitor.visited.ecs_fields.get("log.origin.file.name"),
            Some(&Value::from("user.rs"))
        );
        assert_eq!(
            visitor.visited.ecs_fields.get("log.origin.file.line"),
            Some(&Value::from(999_u64))
        );
    }

    #[test]
    fn into_ecs_log_record_carries_all_fields() {
        let mut ecs_fields = HashMap::new();
        ecs_fields.insert("http.request.method".to_string(), Value::from("GET"));

        let mut labels = HashMap::new();
        labels.insert("labels.custom".to_string(), "value".to_string());

        let expected = EcsLogRecord {
            timestamp: "2024-01-15T10:30:00Z".to_string(),
            log_level: "ERROR",
            message: Some("Something went wrong".to_string()),
            ecs_version: "8.11",
            trace_id: None,
            span_id: None,
            service_name: Arc::from("test-service"),
            service_version: Arc::from("1.2.3"),
            service_environment: Some(Arc::from("prod")),
            service_node_name: None,
            event_dataset: None,
            event_module: "my_app::module".to_string(),
            log_logger: Some(Arc::from("my_app::module")),
            error_type: Some("ValidationError".to_string()),
            error_message: Some("Invalid input".to_string()),
            error_stack_trace: Some("at line 42".to_string()),
            ecs_fields: ecs_fields.clone(),
            labels: labels.clone(),
        };

        let visited = VisitedFields {
            message: expected.message.clone(),
            error_type: expected.error_type.clone(),
            error_message: expected.error_message.clone(),
            error_stack_trace: expected.error_stack_trace.clone(),
            trace_id: None,
            span_id: None,
            ecs_fields,
            labels,
        };

        let visitor = EcsLogRecordVisitor::with_visited(
            expected.timestamp.clone(),
            expected.log_level,
            expected.log_logger.clone(),
            Arc::clone(&expected.service_name),
            Arc::clone(&expected.service_version),
            expected.service_environment.clone(),
            expected.service_node_name.clone(),
            expected.event_dataset.clone(),
            expected.event_module.clone(),
            visited,
        );

        let actual: EcsLogRecord = visitor.into();
        assert_eq!(actual, expected);
    }

    #[test]
    fn explicit_error_message_does_not_synthesize_top_level_message() {
        let mut visited = VisitedFields::new();
        visited.record("error.message", Value::from("db down"));
        assert_eq!(visited.error_message.as_deref(), Some("db down"));
        assert_eq!(visited.message, None);
    }
}
