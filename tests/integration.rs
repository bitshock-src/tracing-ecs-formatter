use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Deserialize)]
struct EcsLogRecord {
    #[serde(rename = "@timestamp")]
    timestamp: String,
    #[serde(rename = "log.level")]
    log_level: String,
    message: String,
    #[serde(rename = "ecs.version")]
    ecs_version: String,
    #[serde(rename = "trace.id")]
    trace_id: Option<String>,
    #[serde(rename = "span.id")]
    span_id: Option<String>,
    #[serde(rename = "service.name")]
    service_name: String,
    #[serde(rename = "service.version")]
    service_version: String,
    #[serde(rename = "log.logger", default)]
    log_logger: Option<String>,
    #[serde(rename = "event.module")]
    event_module: String,
    #[serde(rename = "error.type")]
    error_type: Option<String>,
    #[serde(rename = "error.message")]
    error_message: Option<String>,
    #[serde(rename = "error.stack_trace")]
    error_stack_trace: Option<String>,
    #[serde(rename = "log.origin.file.name", default)]
    log_origin_file_name: Option<String>,
    #[serde(rename = "log.origin.file.line", default)]
    log_origin_file_line: Option<u64>,
    #[serde(flatten)]
    labels: HashMap<String, serde_json::Value>,
}

fn expected_record(log_level: &str, event_module: &str) -> EcsLogRecord {
    let message = match log_level {
        "ERROR" => "error message",
        "WARN" => "warn message",
        "INFO" => "info message",
        "DEBUG" => "debug message",
        "TRACE" => "trace message",
        _ => panic!("unexpected log level: {log_level}"),
    };

    let (error_type, error_message, error_stack_trace) = if log_level == "ERROR" {
        (
            Some("TestError".to_string()),
            Some("something went wrong".to_string()),
            Some("frame1\nframe2".to_string()),
        )
    } else {
        (None, None, None)
    };

    let mut labels = HashMap::from([
        (
            "labels.custom_field".to_string(),
            serde_json::Value::String("custom_value".to_string()),
        ),
        (
            "event.dataset".to_string(),
            serde_json::Value::String("test-service".to_string()),
        ),
    ]);
    if log_level == "INFO" {
        labels.insert(
            "http.request.method".to_string(),
            serde_json::Value::String("GET".to_string()),
        );
        labels.insert(
            "http.response.status_code".to_string(),
            serde_json::Value::from(200_u64),
        );
        labels.insert(
            "url.path".to_string(),
            serde_json::Value::String("/users".to_string()),
        );
    }

    EcsLogRecord {
        timestamp: String::new(),
        log_level: log_level.to_string(),
        message: message.to_string(),
        ecs_version: "8.11".to_string(),
        trace_id: None,
        span_id: None,
        service_name: "test-service".to_string(),
        service_version: "1.0.0".to_string(),
        log_logger: None,
        event_module: event_module.to_string(),
        error_type,
        error_message,
        error_stack_trace,
        log_origin_file_name: None,
        log_origin_file_line: None,
        labels,
    }
}

fn assert_origin(record: &EcsLogRecord, expected_file_suffix: &str) {
    let file = record
        .log_origin_file_name
        .as_deref()
        .expect("log.origin.file.name missing");
    assert!(
        file.ends_with(expected_file_suffix),
        "expected file to end with {expected_file_suffix}, got {file}"
    );
    let line = record
        .log_origin_file_line
        .expect("log.origin.file.line missing");
    assert!(line > 0, "expected non-zero line, got {line}");
}

fn run_example_raw(name: &str, opentelemetry: bool) -> Vec<String> {
    let mut build = escargot::CargoBuild::new().example(name);

    build = if opentelemetry {
        build.features("opentelemetry")
    } else {
        build.no_default_features()
    };

    let example = build.run().expect("failed to build example");

    let output = example.command().output().expect("failed to run example");

    let stdout = String::from_utf8_lossy(&output.stdout);

    stdout
        .lines()
        .filter(|line| line.starts_with('{'))
        .map(str::to_string)
        .collect()
}

fn run_example(name: &str, opentelemetry: bool) -> Vec<EcsLogRecord> {
    run_example_raw(name, opentelemetry)
        .into_iter()
        .map(|line| {
            serde_json::from_str(&line)
                .unwrap_or_else(|e| panic!("Failed to parse log line: {e}\nLine: {line}"))
        })
        .collect()
}

#[test]
fn test_basic_tracing_subscriber() {
    let mut records = run_example("basic", false);

    assert_eq!(records.len(), 5);

    let mut expected = expected_record("ERROR", "basic");
    let record = records.remove(0);
    expected.timestamp = record.timestamp.clone();
    expected.log_origin_file_name = record.log_origin_file_name.clone();
    expected.log_origin_file_line = record.log_origin_file_line;
    assert!(record.trace_id.is_none(), "Expected None trace_id");
    assert!(record.span_id.is_none(), "Expected None span_id");
    assert_origin(&record, "examples/basic.rs");
    assert_eq!(record, expected);

    let mut expected = expected_record("WARN", "basic");
    let record = records.remove(0);
    expected.timestamp = record.timestamp.clone();
    expected.log_origin_file_name = record.log_origin_file_name.clone();
    expected.log_origin_file_line = record.log_origin_file_line;
    assert!(record.trace_id.is_none(), "Expected None trace_id");
    assert!(record.span_id.is_none(), "Expected None span_id");
    assert_origin(&record, "examples/basic.rs");
    assert_eq!(record, expected);

    let mut expected = expected_record("INFO", "basic");
    let record = records.remove(0);
    expected.timestamp = record.timestamp.clone();
    expected.log_origin_file_name = record.log_origin_file_name.clone();
    expected.log_origin_file_line = record.log_origin_file_line;
    assert!(record.trace_id.is_none(), "Expected None trace_id");
    assert!(record.span_id.is_none(), "Expected None span_id");
    assert_origin(&record, "examples/basic.rs");
    assert_eq!(record, expected);

    let mut expected = expected_record("DEBUG", "basic");
    let record = records.remove(0);
    expected.timestamp = record.timestamp.clone();
    expected.log_origin_file_name = record.log_origin_file_name.clone();
    expected.log_origin_file_line = record.log_origin_file_line;
    assert!(record.trace_id.is_none(), "Expected None trace_id");
    assert!(record.span_id.is_none(), "Expected None span_id");
    assert_origin(&record, "examples/basic.rs");
    assert_eq!(record, expected);

    let mut expected = expected_record("TRACE", "basic");
    let record = records.remove(0);
    expected.timestamp = record.timestamp.clone();
    expected.log_origin_file_name = record.log_origin_file_name.clone();
    expected.log_origin_file_line = record.log_origin_file_line;
    assert!(record.trace_id.is_none(), "Expected None trace_id");
    assert!(record.span_id.is_none(), "Expected None span_id");
    assert_origin(&record, "examples/basic.rs");
    assert_eq!(record, expected);
}

#[test]
fn test_opentelemetry_integration() {
    let mut records = run_example("opentelemetry", true);

    assert_eq!(records.len(), 5);

    let mut expected = expected_record("ERROR", "opentelemetry");
    let record = records.remove(0);
    expected.timestamp = record.timestamp.clone();
    expected.trace_id = record.trace_id.clone();
    expected.span_id = record.span_id.clone();
    expected.log_origin_file_name = record.log_origin_file_name.clone();
    expected.log_origin_file_line = record.log_origin_file_line;
    assert!(record.trace_id.is_some(), "Expected trace_id");
    assert!(record.span_id.is_some(), "Expected span_id");
    assert_origin(&record, "examples/opentelemetry.rs");
    assert_eq!(record, expected);

    let mut expected = expected_record("WARN", "opentelemetry");
    let record = records.remove(0);
    expected.timestamp = record.timestamp.clone();
    expected.trace_id = record.trace_id.clone();
    expected.span_id = record.span_id.clone();
    expected.log_origin_file_name = record.log_origin_file_name.clone();
    expected.log_origin_file_line = record.log_origin_file_line;
    assert!(record.trace_id.is_some(), "Expected trace_id");
    assert!(record.span_id.is_some(), "Expected span_id");
    assert_origin(&record, "examples/opentelemetry.rs");
    assert_eq!(record, expected);

    let mut expected = expected_record("INFO", "opentelemetry");
    let record = records.remove(0);
    expected.timestamp = record.timestamp.clone();
    expected.trace_id = record.trace_id.clone();
    expected.span_id = record.span_id.clone();
    expected.log_origin_file_name = record.log_origin_file_name.clone();
    expected.log_origin_file_line = record.log_origin_file_line;
    assert!(record.trace_id.is_some(), "Expected trace_id");
    assert!(record.span_id.is_some(), "Expected span_id");
    assert_origin(&record, "examples/opentelemetry.rs");
    assert_eq!(record, expected);

    let mut expected = expected_record("DEBUG", "opentelemetry");
    let record = records.remove(0);
    expected.timestamp = record.timestamp.clone();
    expected.trace_id = record.trace_id.clone();
    expected.span_id = record.span_id.clone();
    expected.log_origin_file_name = record.log_origin_file_name.clone();
    expected.log_origin_file_line = record.log_origin_file_line;
    assert!(record.trace_id.is_some(), "Expected trace_id");
    assert!(record.span_id.is_some(), "Expected span_id");
    assert_origin(&record, "examples/opentelemetry.rs");
    assert_eq!(record, expected);

    let mut expected = expected_record("TRACE", "opentelemetry");
    let record = records.remove(0);
    expected.timestamp = record.timestamp.clone();
    expected.trace_id = record.trace_id.clone();
    expected.span_id = record.span_id.clone();
    expected.log_origin_file_name = record.log_origin_file_name.clone();
    expected.log_origin_file_line = record.log_origin_file_line;
    assert!(record.trace_id.is_some(), "Expected trace_id");
    assert!(record.span_id.is_some(), "Expected span_id");
    assert_origin(&record, "examples/opentelemetry.rs");
    assert_eq!(record, expected);
}

#[test]
fn test_service_metadata_builders() {
    let records = run_example("service_metadata", false);
    assert_eq!(records.len(), 1);
    let record = &records[0];

    assert_eq!(
        record.labels.get("service.environment"),
        Some(&serde_json::Value::String("prod".to_string()))
    );
    assert_eq!(
        record.labels.get("service.node.name"),
        Some(&serde_json::Value::String("node-7".to_string()))
    );
    assert_eq!(
        record.labels.get("event.dataset"),
        Some(&serde_json::Value::String("svc.access".to_string()))
    );
    assert_origin(record, "examples/service_metadata.rs");
}

#[test]
fn test_first_four_keys_appear_in_spec_order() {
    let lines = run_example_raw("basic", false);
    let line = lines.first().expect("expected at least one log line");

    let ts_pos = line
        .find("\"@timestamp\"")
        .expect("no @timestamp key in output");
    let lvl_pos = line
        .find("\"log.level\"")
        .expect("no log.level key in output");
    let msg_pos = line.find("\"message\"").expect("no message key in output");
    let ecs_pos = line
        .find("\"ecs.version\"")
        .expect("no ecs.version key in output");

    assert!(ts_pos < lvl_pos, "@timestamp must precede log.level");
    assert!(lvl_pos < msg_pos, "log.level must precede message");
    assert!(msg_pos < ecs_pos, "message must precede ecs.version");
}
