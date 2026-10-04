//! PostgreSQL CRUD for `compute_instance`.

use sqlx::PgPool;

use crate::RepoResult;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ComputeInstanceRow {
    pub tenant_id: String,
    pub instance_id: String,
    pub owned_by: String,
    pub purpose: String,
    pub offering_id: Option<String>,
    pub node_id: Option<String>,
    pub vcpu: i32,
    pub memory_mb: i64,
    pub disk_mb: i64,
    pub assurance_level: String,
    pub state: String,
    pub expires_at: Option<String>,
    pub renewed_until: Option<String>,
    pub endpoints: serde_json::Value,
    pub metrics: serde_json::Value,
    pub workspace_ref: Option<String>,
    pub last_failure: Option<String>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

const INSTANCE_COLUMNS: &str = "tenant_id, instance_id, owned_by, purpose, offering_id, node_id, vcpu, memory_mb, disk_mb, assurance_level, state, expires_at::text AS expires_at, renewed_until::text AS renewed_until, endpoints::text AS endpoints, metrics::text AS metrics, workspace_ref, last_failure, version, created_at::text AS created_at, updated_at::text AS updated_at, deleted_at::text AS deleted_at";

pub async fn insert_instance(
    pool: &PgPool,
    tenant_id: &str,
    instance_id: &str,
    owned_by: &str,
    purpose: &str,
    offering_id: Option<&str>,
    vcpu: i32,
    memory_mb: i64,
    disk_mb: i64,
    assurance_level: &str,
) -> RepoResult<()> {
    sqlx::query(
        "INSERT INTO compute_instance (tenant_id, instance_id, owned_by, purpose, offering_id, vcpu, memory_mb, disk_mb, assurance_level, state, version, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'requested', 0, now(), now())",
    )
    .bind(tenant_id)
    .bind(instance_id)
    .bind(owned_by)
    .bind(purpose)
    .bind(offering_id)
    .bind(vcpu)
    .bind(memory_mb)
    .bind(disk_mb)
    .bind(assurance_level)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_instance(
    pool: &PgPool,
    tenant_id: &str,
    instance_id: &str,
) -> RepoResult<Option<ComputeInstanceRow>> {
    let query = format!(
        "SELECT {INSTANCE_COLUMNS} FROM compute_instance WHERE tenant_id = $1 AND instance_id = $2 AND deleted_at IS NULL"
    );
    let row = sqlx::query_as::<_, ComputeInstanceRow>(&query)
        .bind(tenant_id)
        .bind(instance_id)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn update_instance_state(
    pool: &PgPool,
    tenant_id: &str,
    instance_id: &str,
    state: &str,
    node_id: Option<&str>,
) -> RepoResult<()> {
    sqlx::query(
        "UPDATE compute_instance SET state = $3, node_id = COALESCE($4, node_id), version = version + 1, updated_at = now() WHERE tenant_id = $1 AND instance_id = $2 AND deleted_at IS NULL",
    )
    .bind(tenant_id)
    .bind(instance_id)
    .bind(state)
    .bind(node_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_expired_instances(
    pool: &PgPool,
    now: &str,
    limit: i64,
) -> RepoResult<Vec<ComputeInstanceRow>> {
    let query = format!(
        "SELECT {INSTANCE_COLUMNS} FROM compute_instance WHERE state = 'active' AND expires_at IS NOT NULL AND expires_at < $1::timestamptz AND deleted_at IS NULL ORDER BY expires_at ASC LIMIT $2"
    );
    let rows = sqlx::query_as::<_, ComputeInstanceRow>(&query)
        .bind(now)
        .bind(limit)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}
