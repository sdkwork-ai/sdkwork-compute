//! `ComputeRepository` port implementation over the sqlx free functions.
//!
//! Maps the row types to the domain model and delegates SQL to the sibling
//! modules, so the service crate stays storage-agnostic.

use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

use sdkwork_compute_service::model::{
    ComputeAllocation, ComputeInstance, ComputeNode, ComputeOffering,
};
use sdkwork_compute_service::ComputeRepository;

use crate::allocation;
use crate::instance;
use crate::node;

/// PostgreSQL implementation of the compute repository port.
#[derive(Clone)]
pub struct PgComputeRepository {
    pool: Arc<PgPool>,
}

impl PgComputeRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

fn capabilities_to_vec(value: &serde_json::Value) -> Vec<String> {
    serde_json::from_value::<Vec<String>>(value.clone()).unwrap_or_default()
}

fn row_to_node(row: node::ComputeNodeRow) -> ComputeNode {
    ComputeNode {
        tenant_id: row.tenant_id,
        node_id: row.node_id,
        display_name: row.display_name,
        node_kind: row.node_kind,
        dispatch_endpoint: row.dispatch_endpoint,
        region: row.region,
        assurance_capabilities: capabilities_to_vec(&row.assurance_capabilities),
        capacity_vcpu: row.capacity_vcpu,
        capacity_memory_mb: row.capacity_memory_mb,
        capacity_disk_mb: row.capacity_disk_mb,
        max_units: row.max_units,
        status: row.status,
        last_heartbeat_at: row.last_heartbeat_at,
        health_state: row.health_state,
        metadata: row.metadata,
        version: row.version,
        created_at: row.created_at,
        updated_at: row.updated_at,
        deleted_at: row.deleted_at,
    }
}

fn row_to_instance(row: instance::ComputeInstanceRow) -> ComputeInstance {
    ComputeInstance {
        tenant_id: row.tenant_id,
        instance_id: row.instance_id,
        owned_by: row.owned_by,
        purpose: row.purpose,
        offering_id: row.offering_id,
        node_id: row.node_id,
        vcpu: row.vcpu,
        memory_mb: row.memory_mb,
        disk_mb: row.disk_mb,
        assurance_level: row.assurance_level,
        state: row.state,
        expires_at: row.expires_at,
        renewed_until: row.renewed_until,
        endpoints: serde_json::from_value::<Vec<serde_json::Value>>(row.endpoints.clone())
            .unwrap_or_default(),
        metrics: row.metrics,
        workspace_ref: row.workspace_ref,
        last_failure: row.last_failure,
        version: row.version,
        created_at: row.created_at,
        updated_at: row.updated_at,
        deleted_at: row.deleted_at,
    }
}

fn row_to_allocation(row: allocation::ComputeAllocationRow) -> ComputeAllocation {
    ComputeAllocation {
        tenant_id: row.tenant_id,
        allocation_id: row.allocation_id,
        consumer_domain: row.consumer_domain,
        consumer_ref: row.consumer_ref,
        instance_id: row.instance_id,
        node_id: row.node_id,
        requested_target: row.requested_target,
        effective_target: row.effective_target.unwrap_or_default(),
        allocation_state: row.allocation_state,
        lease_owner: row.lease_owner,
        lease_expires_at: row.lease_expires_at,
        fencing_generation: row.fencing_generation,
        is_current: row.is_current,
        version: row.version,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

#[async_trait]
impl ComputeRepository for PgComputeRepository {
    async fn upsert_node(&self, node: ComputeNode) {
        let _ = node::upsert_node(
            &self.pool,
            &node.tenant_id,
            &node.node_id,
            node.display_name.as_deref(),
            &node.node_kind,
            &node.dispatch_endpoint,
            node.region.as_deref(),
            &serde_json::to_value(&node.assurance_capabilities).unwrap_or_default(),
            node.capacity_vcpu,
            node.capacity_memory_mb,
            node.capacity_disk_mb,
            node.max_units,
            &node.status,
            &node.metadata,
        )
        .await;
    }

    async fn get_node(&self, tenant_id: &str, node_id: &str) -> Option<ComputeNode> {
        node::get_node(&self.pool, tenant_id, node_id)
            .await
            .ok()
            .flatten()
            .map(row_to_node)
    }

    async fn list_eligible_nodes(
        &self,
        tenant_id: &str,
        node_kind: Option<&str>,
    ) -> Vec<ComputeNode> {
        node::list_eligible_nodes(&self.pool, tenant_id, node_kind)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(row_to_node)
            .collect()
    }

    async fn upsert_offering(&self, offering: ComputeOffering) {
        let _ = sqlx::query(
            "INSERT INTO compute_offering (tenant_id, compute_offering_id, display_name, vcpu, memory_mb, disk_mb, assurance_level, unit_basis, status, version, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 0, now(), now()) ON CONFLICT (tenant_id, compute_offering_id) DO UPDATE SET display_name = EXCLUDED.display_name, vcpu = EXCLUDED.vcpu, memory_mb = EXCLUDED.memory_mb, disk_mb = EXCLUDED.disk_mb, assurance_level = EXCLUDED.assurance_level, unit_basis = EXCLUDED.unit_basis, status = EXCLUDED.status, version = compute_offering.version + 1, updated_at = now()",
        )
        .bind(&offering.tenant_id)
        .bind(&offering.offering_id)
        .bind(offering.display_name.as_deref())
        .bind(offering.vcpu)
        .bind(offering.memory_mb)
        .bind(offering.disk_mb)
        .bind(&offering.assurance_level)
        .bind(&offering.unit_basis)
        .bind(&offering.status)
        .execute(&*self.pool)
        .await;
    }

    async fn get_offering(&self, tenant_id: &str, offering_id: &str) -> Option<ComputeOffering> {
        let row = sqlx::query_as::<_, ComputeOfferingRow>(
            "SELECT tenant_id, compute_offering_id AS offering_id, display_name, vcpu, memory_mb, disk_mb, assurance_level, unit_basis, status, version, created_at::text AS created_at, updated_at::text AS updated_at, deleted_at::text AS deleted_at FROM compute_offering WHERE tenant_id = $1 AND compute_offering_id = $2 AND deleted_at IS NULL",
        )
        .bind(tenant_id)
        .bind(offering_id)
        .fetch_optional(&*self.pool)
        .await
        .ok()
        .flatten()?;
        Some(ComputeOffering {
            tenant_id: row.tenant_id,
            offering_id: row.offering_id,
            display_name: row.display_name,
            vcpu: row.vcpu,
            memory_mb: row.memory_mb,
            disk_mb: row.disk_mb,
            assurance_level: row.assurance_level,
            unit_basis: row.unit_basis,
            status: row.status,
            version: row.version,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
        })
    }

    async fn upsert_instance(&self, instance: ComputeInstance) {
        let _ = instance::update_instance_state(
            &self.pool,
            &instance.tenant_id,
            &instance.instance_id,
            &instance.state,
            instance.node_id.as_deref(),
        )
        .await;
    }

    async fn get_instance(&self, tenant_id: &str, instance_id: &str) -> Option<ComputeInstance> {
        instance::get_instance(&self.pool, tenant_id, instance_id)
            .await
            .ok()
            .flatten()
            .map(row_to_instance)
    }

    async fn upsert_allocation(&self, allocation: ComputeAllocation) {
        let _ = allocation::insert_allocation(
            &self.pool,
            &allocation.tenant_id,
            &allocation.allocation_id,
            &allocation.consumer_domain,
            &allocation.consumer_ref,
            allocation.instance_id.as_deref(),
            allocation.node_id.as_deref(),
            allocation.requested_target.as_deref(),
            &allocation.effective_target,
            allocation.lease_owner.as_deref(),
            allocation.lease_expires_at.as_deref(),
        )
        .await;
    }

    async fn get_allocation(
        &self,
        tenant_id: &str,
        allocation_id: &str,
    ) -> Option<ComputeAllocation> {
        allocation::get_allocation(&self.pool, tenant_id, allocation_id)
            .await
            .ok()
            .flatten()
            .map(row_to_allocation)
    }

    async fn list_expired_allocations(
        &self,
        tenant_id: &str,
        as_of: &str,
        limit: usize,
    ) -> Vec<ComputeAllocation> {
        // Tenant filter rides in SQL for the PostgreSQL sweep.
        allocation::list_expired_allocations(&self.pool, tenant_id, as_of, limit as i64)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(row_to_allocation)
            .collect()
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ComputeOfferingRow {
    tenant_id: String,
    offering_id: String,
    display_name: Option<String>,
    vcpu: i32,
    memory_mb: i64,
    disk_mb: i64,
    assurance_level: String,
    unit_basis: String,
    status: String,
    version: i64,
    created_at: String,
    updated_at: String,
    deleted_at: Option<String>,
}
