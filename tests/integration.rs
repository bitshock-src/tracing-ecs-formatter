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
    #[serde(rename = "log.logger")]
    log_logger: String,
    #[serde(rename = "error.type")]
    error_type: Option<String>,
    #[serde(rename = "error.message")]
    error_message: Option<String>,
    #[serde(rename = "error.stack_trace")]
    error_stack_trace: Option<String>,
    #[serde(flatten)]
    labels: HashMap<String, serde_json::Value>,
}

fn expected_record(log_level: &str, log_logger: &str) -> EcsLogRecord {
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

    let labels = HashMap::from([(
        "labels.custom_field".to_string(),
        serde_json::Value::String("custom_value".to_string()),
    )]);

    EcsLogRecord {
        timestamp: String::new(),
        log_level: log_level.to_string(),
        message: message.to_string(),
        ecs_version: "8.11".to_string(),
        trace_id: None,
        span_id: None,
        service_name: "test-service".to_string(),
        service_version: "1.0.0".to_string(),
        log_logger: log_logger.to_string(),
        error_type,
        error_message,
        error_stack_trace,
        labels,
    }
}

fn run_example(name: &str, opentelemetry: bool) -> Vec<EcsLogRecord> {
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
        .map(|line| {
            serde_json::from_str(line)
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
    assert!(record.trace_id.is_none(), "Expected None trace_id");
    assert!(record.span_id.is_none(), "Expected None span_id");
    assert_eq!(record, expected);

    let mut expected = expected_record("WARN", "basic");
    let record = records.remove(0);
    expected.timestamp = record.timestamp.clone();
    assert!(record.trace_id.is_none(), "Expected None trace_id");
    assert!(record.span_id.is_none(), "Expected None span_id");
    assert_eq!(record, expected);

    let mut expected = expected_record("INFO", "basic");
    let record = records.remove(0);
    expected.timestamp = record.timestamp.clone();
    assert!(record.trace_id.is_none(), "Expected None trace_id");
    assert!(record.span_id.is_none(), "Expected None span_id");
    assert_eq!(record, expected);

    let mut expected = expected_record("DEBUG", "basic");
    let record = records.remove(0);
    expected.timestamp = record.timestamp.clone();
    assert!(record.trace_id.is_none(), "Expected None trace_id");
    assert!(record.span_id.is_none(), "Expected None span_id");
    assert_eq!(record, expected);

    let mut expected = expected_record("TRACE", "basic");
    let record = records.remove(0);
    expected.timestamp = record.timestamp.clone();
    assert!(record.trace_id.is_none(), "Expected None trace_id");
    assert!(record.span_id.is_none(), "Expected None span_id");
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
    assert!(record.trace_id.is_some(), "Expected trace_id");
    assert!(record.span_id.is_some(), "Expected span_id");
    assert_eq!(record, expected);

    let mut expected = expected_record("WARN", "opentelemetry");
    let record = records.remove(0);
    expected.timestamp = record.timestamp.clone();
    expected.trace_id = record.trace_id.clone();
    expected.span_id = record.span_id.clone();
    assert!(record.trace_id.is_some(), "Expected trace_id");
    assert!(record.span_id.is_some(), "Expected span_id");
    assert_eq!(record, expected);

    let mut expected = expected_record("INFO", "opentelemetry");
    let record = records.remove(0);
    expected.timestamp = record.timestamp.clone();
    expected.trace_id = record.trace_id.clone();
    expected.span_id = record.span_id.clone();
    assert!(record.trace_id.is_some(), "Expected trace_id");
    assert!(record.span_id.is_some(), "Expected span_id");
    assert_eq!(record, expected);

    let mut expected = expected_record("DEBUG", "opentelemetry");
    let record = records.remove(0);
    expected.timestamp = record.timestamp.clone();
    expected.trace_id = record.trace_id.clone();
    expected.span_id = record.span_id.clone();
    assert!(record.trace_id.is_some(), "Expected trace_id");
    assert!(record.span_id.is_some(), "Expected span_id");
    assert_eq!(record, expected);

    let mut expected = expected_record("TRACE", "opentelemetry");
    let record = records.remove(0);
    expected.timestamp = record.timestamp.clone();
    expected.trace_id = record.trace_id.clone();
    expected.span_id = record.span_id.clone();
    assert!(record.trace_id.is_some(), "Expected trace_id");
    assert!(record.span_id.is_some(), "Expected span_id");
    assert_eq!(record, expected);
}
