# tracing-ecs-formatter

OpenTelemetry-aware ECS JSON formatter for [`tracing-subscriber`](https://docs.rs/tracing-subscriber).

Every log line carries the active `trace.id` and `span.id` from the OpenTelemetry context, so a log entry links directly to the distributed trace that produced it. Output conforms to the [ECS 8.x](https://www.elastic.co/docs/reference/ecs) and [ecs-logging](https://github.com/elastic/ecs-logging) specs.

## Features

- **Log↔trace correlation out of the box** — pulls `trace.id` and `span.id` from the current OpenTelemetry span; no manual plumbing at each log site.
- **ECS 8.x compliant** — fields land at their canonical ECS paths, matching what ECS-aware dashboards, visualisations, and detection rules query.
- **Lightweight** — implements `FormatEvent`, composes with existing `tracing-subscriber` layers.
- **Optional OTel dep** — the OpenTelemetry integration is a default feature; turn it off for a smaller dependency tree if you don't need trace correlation.
- **Service metadata** — `service.name`, `service.version`, plus optional `service.environment`, `service.node.name`, and `event.dataset` (defaults to `service.name` per the ecs-logging spec).

## Installation

```toml
[dependencies]
tracing-ecs-formatter = "2"
```

Without OpenTelemetry support (smaller dependency tree):

```toml
[dependencies]
tracing-ecs-formatter = { version = "2", default-features = false }
```

## Usage

```rust
use tracing_subscriber::fmt;
use tracing_ecs_formatter::EcsFormatter;

fn main() {
    tracing_subscriber::fmt()
        .event_format(EcsFormatter::new("my-service", "1.0.0"))
        .init();

    tracing::info!("Application started");
}
```

## Output

Each log event produces a single JSON line:

```json
{"@timestamp":"2024-01-15T10:30:00.000Z","log.level":"INFO","message":"Application started","ecs.version":"8.11","service.name":"my-service","service.version":"1.0.0","log.logger":"my_app"}
```

## Field Routing

Field names are routed by prefix:

1. `message`, `error.type`, `error.message`, `error.stack_trace` populate the dedicated ECS slots of the same name.
2. Any name starting with an ECS namespace (`http.`, `url.`, `client.`, `event.`, `user_agent.`, `custom.`, …) is emitted as a top-level dotted key with its JSON type preserved (integers, floats, booleans, and strings).
3. Everything else goes to `labels.<sanitized-name>` as a string. Per the ECS logging spec, `.`, `*`, and `\` in label keys are replaced with `_`.

```rust
tracing::info!(
    http.request.method = "GET",
    http.response.status_code = 201_u64,
    url.path = "/users",
    client.ip = "203.0.113.5",
    user_id = 42,
    "request handled"
);
```

produces:

```json
{
  "@timestamp": "…", "log.level": "INFO", "message": "request handled",
  "ecs.version": "8.11", "service.name": "my-service", "service.version": "1.0.0",
  "log.logger": "my_app",
  "http.request.method": "GET",
  "http.response.status_code": 201,
  "url.path": "/users",
  "client.ip": "203.0.113.5",
  "labels.user_id": "42"
}
```

An ECS-aware search backend indexes `"http.request.method": "GET"` at the same path as the nested form `{"http": {"request": {"method": "GET"}}}`, matching what downstream dashboards and detection rules query at the canonical ECS paths.

Field names whose prefix is not in the ECS namespace list — including typos like `htp.request.method` — fall through to `labels.htp_request_method`, so the index mapping stays confined to the ECS namespace allowlist.

## ECS Fields

| Field                         | Description                                             |
|-------------------------------|---------------------------------------------------------|
| `@timestamp`                  | RFC 3339 timestamp                                      |
| `log.level`                   | ERROR, WARN, INFO, DEBUG, or TRACE                      |
| `message`                     | Log message                                             |
| `ecs.version`                 | Always "8.11"                                           |
| `service.name`                | Configured service name                                 |
| `service.version`             | Configured service version                              |
| `service.environment`         | Optional; set via `.with_service_environment(…)`        |
| `service.node.name`           | Optional; set via `.with_service_node_name(…)`          |
| `event.dataset`               | Defaults to `service.name`; override via `.with_event_dataset(…)` |
| `log.logger`                  | Tracing target (logger instance name)                   |
| `trace.id`                    | OpenTelemetry trace ID (when available)                 |
| `span.id`                     | OpenTelemetry span ID (when available)                  |
| `error.type`                  | Error type (when present)                               |
| `error.message`               | Error message (when present)                            |
| `error.stack_trace`           | Stack trace (when present)                              |
| `<ecs-namespace>.<name>`      | Any ECS-namespaced field, typed                         |
| `labels.<sanitized-name>`     | Anything else, string-valued (per ECS logging spec)     |

## Builder API

```rust
use tracing_ecs_formatter::EcsFormatter;

fn main() {
    let formatter = EcsFormatter::new("my-service", "1.0.0")
        .with_service_environment("prod")
        .with_service_node_name("web-7")
        .with_event_dataset("my-service.access");
}
```

## OpenTelemetry Integration

When used with `tracing-opentelemetry`, trace context is automatically included:

```rust
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_ecs_formatter::EcsFormatter;

fn main() {
    let tracer = opentelemetry::trace::noop::NoopTracer::new();

    let _ = tracing_subscriber::registry()
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(
            tracing_subscriber::fmt::layer()
                .event_format(EcsFormatter::new("my-service", "1.0.0"))
        )
        .try_init();
}
```

## Error Fields

Use dotted field names to populate ECS error fields:

```rust
fn main() {
    let stacktrace = "at db::connect (db.rs:42)\nat main (main.rs:10)";
    tracing::error!(
        error.type = "DatabaseError",
        error.message = "Connection refused",
        error.stack_trace = %stacktrace,
        "Failed to connect to database"
    );
}
```

## Migrating from 1.x

Version 2.0 changes the JSON shape to be ECS-compliant. If you were on 1.x:

- **HTTP / URL / client / event / etc. fields moved to the JSON root.** In 1.x, `http.request.method = "GET"` was emitted as `"labels.http.request.method": "GET"`, which an ECS-aware backend indexed as a nested object under `labels` — violating the ECS `labels` contract (keyword-only, no nested objects). 2.0 emits `"http.request.method": "GET"`, which lands at the canonical ECS path.
- **Numeric and boolean ECS values keep their JSON types.** `http.response.status_code = 201u16` is now the JSON number `201` (not `"201"`), enabling range queries, histograms, and aggregations.
- **Label keys are sanitized.** Per the ECS logging spec, `.`, `*`, `\` in label keys are replaced with `_`. Non-ECS-namespaced fields with dots (e.g. `foo.bar = 1` where `foo` is not an ECS namespace) now emit `"labels.foo_bar": "1"` instead of `"labels.foo.bar": "1"`.
- **`labels.*` values are always strings.** Per the ECS `labels` contract (`object_type: keyword`).

Any downstream pipeline that had rename rules working around the 1.x `labels.http.*` shape should remove them.

## Feature Flags

| Feature         | Default | Description                                                            |
|-----------------|---------|------------------------------------------------------------------------|
| `opentelemetry` | Yes     | Enables `trace.id` and `span.id` extraction from OpenTelemetry context |

## License

Licensed under [Apache License, Version 2.0](https://github.com/bitshock-src/tracing-ecs-formatter/blob/main/LICENSE).
