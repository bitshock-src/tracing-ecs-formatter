# tracing-ecs-formatter

ECS (Elastic Common Schema) 8.11 JSON formatter for [`tracing-subscriber`](https://docs.rs/tracing-subscriber).

Produces JSON logs conforming to the [ECS 8.11 specification](https://www.elastic.co/guide/en/ecs/8.11/index.html), suitable for ingestion into Elasticsearch, Kibana, and other ECS-compatible systems.

## Features

- **ECS 8.11 compliant** — Explicitly targets the latest ECS specification
- **OpenTelemetry integration** — Automatically extracts `trace.id` and `span.id` from OpenTelemetry context
- **Lightweight** — Implements `FormatEvent` trait, composes with existing `tracing-subscriber` layers
- **Minimal dependencies** — OpenTelemetry support is optional
- **Service metadata** — Built-in `service.name` and `service.version` fields

## Installation

```toml
[dependencies]
tracing-ecs-formatter = "1"
```

Without OpenTelemetry support (smaller dependency tree):

```toml
[dependencies]
tracing-ecs-formatter = { version = "1", default-features = false }
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

## ECS Fields

| Field               | Description                             |
|---------------------|-----------------------------------------|
| `@timestamp`        | RFC 3339 timestamp                      |
| `log.level`         | ERROR, WARN, INFO, DEBUG, or TRACE      |
| `message`           | Log message                             |
| `ecs.version`       | Always "8.11"                           |
| `service.name`      | Configured service name                 |
| `service.version`   | Configured service version              |
| `log.logger`        | Tracing target (logger instance name)   |
| `trace.id`          | OpenTelemetry trace ID (when available) |
| `span.id`           | OpenTelemetry span ID (when available)  |
| `error.type`        | Error type (when present)               |
| `error.message`     | Error message (when present)            |
| `error.stack_trace` | Stack trace (when present)              |
| `labels.*`          | Additional fields from tracing events   |

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

## Custom Fields

Any additional fields are output as `labels.*`:

```rust
fn main() {
    tracing::info!(
        user_id = 42,
        request_id = "abc-123",
        "User logged in"
    );
}
```

Output:
```json
{"@timestamp":"2024-01-15T10:30:00.000Z","log.level":"INFO","message":"User logged in","ecs.version":"8.11","service.name":"my-service","service.version":"1.0.0","log.logger":"my_app","labels.user_id":"42","labels.request_id":"abc-123"}
```

Works with all field types:

```rust
fn main() {
    let headers = vec![("Content-Type", "application/json")];
    let addr = std::net::Ipv4Addr::new(192, 168, 1, 1);

    // Strings
    tracing::info!(endpoint = "/api/users", "Request received");

    // Numbers (serialized as strings in labels)
    tracing::info!(latency_ms = 150, status_code = 200, "Request completed");

    // Debug formatting
    tracing::info!(?headers, "Processing request");

    // Display formatting
    tracing::info!(client_ip = %addr, "Connection established");
}
```

## Feature Flags

| Feature         | Default | Description                                                            |
|-----------------|---------|------------------------------------------------------------------------|
| `opentelemetry` | Yes     | Enables `trace.id` and `span.id` extraction from OpenTelemetry context |

## License

Licensed under [Apache License, Version 2.0](https://github.com/bitshock-src/tracing-ecs-formatter/blob/main/LICENSE).
