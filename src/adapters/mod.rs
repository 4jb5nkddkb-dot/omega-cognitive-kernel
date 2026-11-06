//! External world adapters (whitelist-only).

pub mod http;
pub mod world;

pub use http::{HttpActionSpec, HttpAdapterConfig, HttpToolAdapter};
pub use world::WorldExecutor;
