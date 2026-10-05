//! PostgreSQL CRUD for `compute_allocation`.

use sqlx::PgPool;

use crate::RepoResult;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ComputeAllocationRow {
    pub tenant_id: String,
    pub allocation_id: String,
    pub consumer_domain: String,
    pub consumer_ref: String,
    pub instance_id: Option<String>,
    pub node_id: Option<String>,
    pub requested_target: Option<String>,
    pub effective_target: Option<String>,
    pub allocation_state: String,
    pub lease_owner: Option<String>,
    pub lease_expires_at: Option<String>,
    pub fencing_generation: i64,
    pub is_current: bool,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

const ALLOCATION_COLUMNS: &str = "tenant_id, allocation_id, consumer_domain, consumer_ref, instance_id, node_id, requested_target, effective_target, allocation_state, lease_owner, lease_expires_at::text AS lease_expires_at, fencing_generation, is_current, version, created_at::text AS created_at, updated_at::text AS updated_at";

#[allow(clippy::too_many_arguments)]
pub async fn insert_allocation(
    pool: &PgPool,
    tenant_id: &str,
    allocation_id: &str,
    consumer_domain: &str,
    consumer_ref: &str,
    instance_id: Option<&str>,
    node_id: Option<&str>,
    requested_target: Option<&str>,
    effective_target: &str,
    lease_owner: Option<&str>,
    lease_expires_at: Option<&str>,
) -> RepoResult<()> {
    sqlx::query(
        "INSERT INTO compute_allocation (tenant_id, allocation_id, consumer_domain, consumer_ref, instance_id, node_id, requested_target, effective_target, allocation_state, lease_owner, lease_expires_at, fencing_generation, is_current, version, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'active', $9, $10::timestamptz, 0, TRUE, 0, now(), now())",
    )
    .bind(tenant_id)
    .bind(allocation_id)
    .bind(consumer_domain)
    .bind(consumer_ref)
    .bind(instance_id)
    .bind(node_id)
    .bind(requested_target)
    .bind(effective_target)
    .bind(lease_owner)
    .bind(lease_expires_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_allocation(
    pool: &PgPool,
    tenant_id: &str,
    allocation_id: &str,
) -> RepoResult<Option<ComputeAllocationRow>> {
    let query = format!(
        "SELECT {ALLOCATION_COLUMNS} FROM compute_allocation WHERE tenant_id = $1 AND allocation_id = $2"
    );
    let row = sqlx::query_as::<_, ComputeAllocationRow>(&query)
        .bind(tenant_id)
        .bind(allocation_id)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn transition_allocation_state(
    pool: &PgPool,
    tenant_id: &str,
    allocation_id: &str,
    new_state: &str,
    expected_version: i64,
) -> RepoResult<bool> {
    let result = sqlx::query(
        "UPDATE compute_allocation SET allocation_state = $3, version = version + 1, updated_at = now() WHERE tenant_id = $1 AND allocation_id = $2 AND version = $4",
    )
    .bind(tenant_id)
    .bind(allocation_id)
    .bind(new_state)
    .bind(expected_version)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn list_expired_allocations(
    pool: &PgPool,
    tenant_id: &str,
    now: &str,
    limit: i64,
) -> RepoResult<Vec<ComputeAllocationRow>> {
    let query = format!(
        "SELECT {ALLOCATION_COLUMNS} FROM compute_allocation WHERE tenant_id = $1 AND allocation_state IN ('requested', 'allocating', 'ready', 'active') AND lease_expires_at IS NOT NULL AND lease_expires_at < $2::timestamptz ORDER BY lease_expires_at ASC LIMIT $3"
    );
    let rows = sqlx::query_as::<_, ComputeAllocationRow>(&query)
        .bind(tenant_id)
        .bind(now)
        .bind(limit)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}
