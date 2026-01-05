use once_cell::sync::Lazy;
use tokio::runtime::{Handle, Runtime};

static RUNTIME: Lazy<Runtime> =
    Lazy::new(|| Runtime::new().expect("failed to start global tokio runtime for device client"));

/// Get a clone of the global Tokio runtime handle
pub fn handle() -> Handle {
    RUNTIME.handle().clone()
}
