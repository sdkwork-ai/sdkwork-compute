//! Compute lease and on-demand allocation domain (REQ-2026-1004 / ADR-20261004).
//!
//! Owns compute nodes, rentable offerings/instances, and consumer
//! allocations. Consumers (agents, sandbox, webserver) hold only
//! allocation/instance references.

pub mod http;
pub mod model;
pub mod repository;
pub mod service;

pub use model::{ComputeAllocation, ComputeInstance, ComputeNode, ComputeOffering};
pub use repository::{ComputeRepository, InMemoryComputeRepository};
pub use service::ComputeService;
