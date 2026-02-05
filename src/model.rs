use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, PartialEq, Serialize)]
pub struct EcsLogRecord {
    #[serde(rename = "@timestamp")]
    pub timestamp: String,
    #[serde(rename = "log.level")]
    pub log_level: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(rename = "ecs.version")]
    pub ecs_version: &'static str,
    #[serde(rename = "trace.id", skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(rename = "span.id", skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
    #[serde(rename = "service.name")]
    pub service_name: Arc<str>,
    #[serde(rename = "service.version")]
    pub service_version: Arc<str>,
    #[serde(rename = "log.logger")]
    pub log_logger: String,
    #[serde(rename = "error.type", skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    #[serde(rename = "error.message", skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(rename = "error.stack_trace", skip_serializing_if = "Option::is_none")]
    pub error_stack_trace: Option<String>,
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub labels: HashMap<String, String>,
}
