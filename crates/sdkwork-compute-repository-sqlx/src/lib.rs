//! PostgreSQL sqlx repository for the compute lease domain.
//!
//! Implements CRUD for compute_node, compute_offering, compute_instance,
//! and compute_allocation tables per the baseline DDL.

pub mod allocation;
pub mod instance;
pub mod node;

use sqlx::PgPool;

/// Errors from the compute PostgreSQL repository.
#[derive(Debug, thiserror::Error)]
pub enum ComputeRepositoryError {
    #[error("compute node not found: {0}")]
    NodeNotFound(String),
    #[error("compute instance not found: {0}")]
    InstanceNotFound(String),
    #[error("compute allocation not found: {0}")]
    AllocationNotFound(String),
    #[error("optimistic version conflict on {0}")]
    VersionConflict(String),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

pub type RepoResult<T> = Result<T, ComputeRepositoryError>;

/// PostgreSQL connection pool wrapper for compute repositories.
#[derive(Clone)]
pub struct ComputePool {
    pool: PgPool,
}

impl ComputePool {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}
