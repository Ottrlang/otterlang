use std::sync::Once;

use tracing_subscriber::{EnvFilter, fmt};

static INIT: Once = Once::new();

/// Initialise tracing subscriber once per process.
pub fn init_logging() {
    INIT.call_once(|| {
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("otterlang=info"));

        fmt()
            .with_env_filter(env_filter)
            .with_target(false)
            .compact()
            .init();
    });
}
