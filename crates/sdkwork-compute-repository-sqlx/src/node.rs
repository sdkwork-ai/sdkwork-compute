//! PostgreSQL CRUD for `compute_node`.

use sqlx::PgPool;

use crate::RepoResult;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ComputeNodeRow {
    pub tenant_id: String,
    pub node_id: String,
    pub display_name: Option<String>,
    pub node_kind: String,
    pub dispatch_endpoint: String,
    pub region: Option<String>,
    pub assurance_capabilities: serde_json::Value,
    pub capacity_vcpu: i32,
    pub capacity_memory_mb: i64,
    pub capacity_disk_mb: i64,
    pub max_units: i32,
    pub status: String,
    pub last_heartbeat_at: Option<String>,
    pub health_state: Option<String>,
    pub metadata: serde_json::Value,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

const NODE_COLUMNS: &str = "tenant_id, node_id, display_name, node_kind, dispatch_endpoint, region, assurance_capabilities::text AS assurance_capabilities, capacity_vcpu, capacity_memory_mb, capacity_disk_mb, max_units, status, last_heartbeat_at::text AS last_heartbeat_at, health_state, metadata::text AS metadata, version, created_at::text AS created_at, updated_at::text AS updated_at, deleted_at::text AS deleted_at";

#[allow(clippy::too_many_arguments)]
pub async fn upsert_node(
    pool: &PgPool,
    tenant_id: &str,
    node_id: &str,
    display_name: Option<&str>,
    node_kind: &str,
    dispatch_endpoint: &str,
    region: Option<&str>,
    assurance_capabilities: &serde_json::Value,
    capacity_vcpu: i32,
    capacity_memory_mb: i64,
    capacity_disk_mb: i64,
    max_units: i32,
    status: &str,
    metadata: &serde_json::Value,
) -> RepoResult<()> {
    sqlx::query(
        "INSERT INTO compute_node (tenant_id, node_id, display_name, node_kind, dispatch_endpoint, region, assurance_capabilities, capacity_vcpu, capacity_memory_mb, capacity_disk_mb, max_units, status, metadata, version, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8, $9, $10, $11, $12, $13::jsonb, 0, now(), now()) ON CONFLICT (tenant_id, node_id) DO UPDATE SET display_name = EXCLUDED.display_name, node_kind = EXCLUDED.node_kind, dispatch_endpoint = EXCLUDED.dispatch_endpoint, region = EXCLUDED.region, assurance_capabilities = EXCLUDED.assurance_capabilities, capacity_vcpu = EXCLUDED.capacity_vcpu, capacity_memory_mb = EXCLUDED.capacity_memory_mb, capacity_disk_mb = EXCLUDED.capacity_disk_mb, max_units = EXCLUDED.max_units, status = EXCLUDED.status, metadata = EXCLUDED.metadata, version = compute_node.version + 1, updated_at = now()",
    )
    .bind(tenant_id)
    .bind(node_id)
    .bind(display_name)
    .bind(node_kind)
    .bind(dispatch_endpoint)
    .bind(region)
    .bind(assurance_capabilities)
    .bind(capacity_vcpu)
    .bind(capacity_memory_mb)
    .bind(capacity_disk_mb)
    .bind(max_units)
    .bind(status)
    .bind(metadata)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_node(
    pool: &PgPool,
    tenant_id: &str,
    node_id: &str,
) -> RepoResult<Option<ComputeNodeRow>> {
    let query = format!(
        "SELECT {NODE_COLUMNS} FROM compute_node WHERE tenant_id = $1 AND node_id = $2 AND deleted_at IS NULL"
    );
    let row = sqlx::query_as::<_, ComputeNodeRow>(&query)
        .bind(tenant_id)
        .bind(node_id)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn list_eligible_nodes(
    pool: &PgPool,
    tenant_id: &str,
    node_kind: Option<&str>,
) -> RepoResult<Vec<ComputeNodeRow>> {
    let query = format!(
        "SELECT {NODE_COLUMNS} FROM compute_node WHERE tenant_id = $1 AND status = 'active' AND deleted_at IS NULL AND ($2::text IS NULL OR node_kind = $2) ORDER BY max_units DESC, node_id"
    );
    let rows = sqlx::query_as::<_, ComputeNodeRow>(&query)
        .bind(tenant_id)
        .bind(node_kind)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn soft_delete_node(pool: &PgPool, tenant_id: &str, node_id: &str) -> RepoResult<()> {
    sqlx::query(
        "UPDATE compute_node SET deleted_at = now(), status = 'disabled', updated_at = now() WHERE tenant_id = $1 AND node_id = $2 AND deleted_at IS NULL",
    )
    .bind(tenant_id)
    .bind(node_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_heartbeat(
    pool: &PgPool,
    tenant_id: &str,
    node_id: &str,
    health_state: &str,
) -> RepoResult<()> {
    sqlx::query(
        "UPDATE compute_node SET last_heartbeat_at = now(), health_state = $3, updated_at = now() WHERE tenant_id = $1 AND node_id = $2 AND deleted_at IS NULL",
    )
    .bind(tenant_id)
    .bind(node_id)
    .bind(health_state)
    .execute(pool)
    .await?;
    Ok(())
}
