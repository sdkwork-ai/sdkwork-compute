//! Re-export of the in-memory compute repository from the service crate.
//!
//! Kept as a separate crate so test consumers can depend on the in-memory
//! implementation without pulling the service internals.

pub use sdkwork_compute_service::repository::InMemoryComputeRepository;
