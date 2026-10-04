//! Compute service: high-level operations over the compute domain.

use crate::model::{ComputeAllocation, ComputeInstance, ComputeNode};
use crate::repository::InMemoryComputeRepository;

/// High-level compute service operating over an in-memory repository.
/// Production deployments replace the repository with a sqlx-backed
/// implementation; the service logic is repository-agnostic.
pub struct ComputeService {
    repo: std::sync::Arc<InMemoryComputeRepository>,
}

impl ComputeService {
    pub fn new(repo: std::sync::Arc<InMemoryComputeRepository>) -> Self {
        Self { repo }
    }

    pub fn repo(&self) -> &InMemoryComputeRepository {
        &self.repo
    }

    pub fn register_node(&self, node: ComputeNode) {
        self.repo.upsert_node(node);
    }

    pub fn get_node(&self, tenant_id: &str, node_id: &str) -> Option<ComputeNode> {
        self.repo.get_node(tenant_id, node_id)
    }

    pub fn list_eligible_nodes(
        &self,
        tenant_id: &str,
        node_kind: Option<&str>,
    ) -> Vec<ComputeNode> {
        self.repo.list_eligible_nodes(tenant_id, node_kind)
    }

    pub fn create_instance(&self, instance: ComputeInstance) {
        self.repo.upsert_instance(instance);
    }

    pub fn get_instance(&self, tenant_id: &str, instance_id: &str) -> Option<ComputeInstance> {
        self.repo.get_instance(tenant_id, instance_id)
    }

    pub fn record_allocation(&self, allocation: ComputeAllocation) {
        self.repo.upsert_allocation(allocation);
    }

    pub fn get_allocation(
        &self,
        tenant_id: &str,
        allocation_id: &str,
    ) -> Option<ComputeAllocation> {
        self.repo.get_allocation(tenant_id, allocation_id)
    }

    /// Expires live allocations whose lease lapsed before `as_of`.
    /// Tolerates per-row races (skips already-reconciled rows).
    pub fn reconcile_expired_allocations(
        &self,
        tenant_id: &str,
        as_of: &str,
        limit: usize,
    ) -> usize {
        let expired = self.repo.list_expired_allocations(as_of, limit);
        let mut reconciled = 0;
        for mut allocation in expired {
            allocation.allocation_state = "expired".to_string();
            allocation.version += 1;
            self.repo.upsert_allocation(allocation);
            reconciled += 1;
        }
        reconciled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::InMemoryComputeRepository;
    use std::sync::Arc;

    fn make_node(tenant: &str, id: &str, status: &str) -> ComputeNode {
        ComputeNode {
            tenant_id: tenant.to_string(),
            node_id: id.to_string(),
            display_name: None,
            node_kind: "docker".to_string(),
            dispatch_endpoint: "unix:///run/docker.sock".to_string(),
            region: None,
            assurance_capabilities: vec!["container".to_string()],
            capacity_vcpu: 8,
            capacity_memory_mb: 16384,
            capacity_disk_mb: 102400,
            max_units: 4,
            status: status.to_string(),
            last_heartbeat_at: None,
            health_state: None,
            metadata: serde_json::json!({}),
            version: 1,
            created_at: "2026-10-04T00:00:00Z".to_string(),
            updated_at: "2026-10-04T00:00:00Z".to_string(),
            deleted_at: None,
        }
    }

    fn make_allocation(tenant: &str, id: &str, expires: &str) -> ComputeAllocation {
        ComputeAllocation {
            tenant_id: tenant.to_string(),
            allocation_id: id.to_string(),
            consumer_domain: "agents".to_string(),
            consumer_ref: "session.test".to_string(),
            instance_id: None,
            node_id: None,
            requested_target: Some("cloud".to_string()),
            effective_target: "cloud".to_string(),
            allocation_state: "active".to_string(),
            lease_owner: Some("worker-1".to_string()),
            lease_expires_at: Some(expires.to_string()),
            fencing_generation: 0,
            is_current: true,
            version: 1,
            created_at: "2026-10-01T00:00:00Z".to_string(),
            updated_at: "2026-10-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn node_registration_and_eligibility() {
        let repo = Arc::new(InMemoryComputeRepository::new());
        let service = ComputeService::new(repo);

        service.register_node(make_node("10", "node.1", "active"));
        service.register_node(make_node("10", "node.2", "draining"));

        let eligible = service.list_eligible_nodes("10", Some("docker"));
        assert_eq!(eligible.len(), 1);
        assert_eq!(eligible[0].node_id, "node.1");
    }

    #[test]
    fn reconcile_expires_lapsed_allocations() {
        let repo = Arc::new(InMemoryComputeRepository::new());
        let service = ComputeService::new(repo);

        service.record_allocation(make_allocation("10", "alloc.1", "2026-10-03T00:00:00Z"));

        let reconciled = service.reconcile_expired_allocations("10", "2026-10-04T00:00:00Z", 10);
        assert_eq!(reconciled, 1);

        let alloc = service
            .get_allocation("10", "alloc.1")
            .expect("allocation exists");
        assert_eq!(alloc.allocation_state, "expired");

        let reconciled_again =
            service.reconcile_expired_allocations("10", "2026-10-04T00:00:00Z", 10);
        assert_eq!(reconciled_again, 0);
    }
}
