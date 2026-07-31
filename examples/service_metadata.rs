//! Demonstrates the optional service metadata builders:
//! `service.environment`, `service.node.name`, and `event.dataset`.

use tracing_ecs_formatter::EcsFormatter;

fn main() {
    tracing_subscriber::fmt()
        .event_format(
            EcsFormatter::new("test-service", "1.0.0")
                .with_service_environment("prod")
                .with_service_node_name("node-7")
                .with_event_dataset("svc.access"),
        )
        .init();

    tracing::info!("hi");
}
