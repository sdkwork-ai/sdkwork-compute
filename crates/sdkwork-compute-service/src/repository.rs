//! Compute repository port + in-memory implementation.
//!
//! The [`ComputeRepository`] trait is the storage port the service depends
//! on (sandbox-module pattern: `Arc<dyn Trait>` with `#[async_trait]`).
//! [`InMemoryComputeRepository`] is the test/lightweight implementation; the
//! sqlx crate provides the PostgreSQL implementation.

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;

use crate::model::{ComputeAllocation, ComputeInstance, ComputeNode, ComputeOffering};

/// Storage port for the compute domain. All operations are tenant-scoped by
/// the `tenant_id` leading argument.
#[async_trait]
pub trait ComputeRepository: Send + Sync {
    async fn upsert_node(&self, node: ComputeNode);
    async fn get_node(&self, tenant_id: &str, node_id: &str) -> Option<ComputeNode>;
    async fn list_eligible_nodes(
        &self,
        tenant_id: &str,
        node_kind: Option<&str>,
    ) -> Vec<ComputeNode>;

    async fn upsert_offering(&self, offering: ComputeOffering);
    async fn get_offering(&self, tenant_id: &str, offering_id: &str) -> Option<ComputeOffering>;

    async fn upsert_instance(&self, instance: ComputeInstance);
    async fn get_instance(&self, tenant_id: &str, instance_id: &str) -> Option<ComputeInstance>;

    async fn upsert_allocation(&self, allocation: ComputeAllocation);
    async fn get_allocation(
        &self,
        tenant_id: &str,
        allocation_id: &str,
    ) -> Option<ComputeAllocation>;
    async fn list_expired_allocations(
        &self,
        tenant_id: &str,
        as_of: &str,
        limit: usize,
    ) -> Vec<ComputeAllocation>;
}

#[derive(Debug)]
pub struct InMemoryComputeRepository {
    nodes: RwLock<HashMap<(String, String), ComputeNode>>,
    offerings: RwLock<HashMap<(String, String), ComputeOffering>>,
    instances: RwLock<HashMap<(String, String), ComputeInstance>>,
    allocations: RwLock<HashMap<(String, String), ComputeAllocation>>,
}

impl InMemoryComputeRepository {
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
            offerings: RwLock::new(HashMap::new()),
            instances: RwLock::new(HashMap::new()),
            allocations: RwLock::new(HashMap::new()),
        }
    }

    fn read_guard<'a, T>(lock: &'a RwLock<T>) -> std::sync::RwLockReadGuard<'a, T> {
        // Standard poison-recovery idiom: the domain data structures have no
        // intra-item invariants that a panic while holding the lock could
        // invalidate, so re-entering the lock is safe.
        lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn write_guard<'a, T>(lock: &'a RwLock<T>) -> std::sync::RwLockWriteGuard<'a, T> {
        lock.write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Default for InMemoryComputeRepository {
    fn default() -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
            offerings: RwLock::new(HashMap::new()),
            instances: RwLock::new(HashMap::new()),
            allocations: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl ComputeRepository for InMemoryComputeRepository {
    async fn upsert_node(&self, node: ComputeNode) {
        let key = (node.tenant_id.clone(), node.node_id.clone());
        Self::write_guard(&self.nodes).insert(key, node);
    }

    async fn get_node(&self, tenant_id: &str, node_id: &str) -> Option<ComputeNode> {
        Self::read_guard(&self.nodes)
            .get(&(tenant_id.to_string(), node_id.to_string()))
            .cloned()
    }

    async fn list_eligible_nodes(
        &self,
        tenant_id: &str,
        node_kind: Option<&str>,
    ) -> Vec<ComputeNode> {
        let nodes = Self::read_guard(&self.nodes);
        let mut rows: Vec<ComputeNode> = nodes
            .iter()
            .filter(|((t, _), n)| {
                t == tenant_id
                    && n.is_scheduler_eligible()
                    && node_kind.is_none_or(|k| n.node_kind == k)
            })
            .map(|(_, n)| n.clone())
            .collect();
        rows.sort_by_key(|n| std::cmp::Reverse(n.max_units));
        rows
    }

    async fn upsert_offering(&self, offering: ComputeOffering) {
        let key = (offering.tenant_id.clone(), offering.offering_id.clone());
        Self::write_guard(&self.offerings).insert(key, offering);
    }

    async fn get_offering(&self, tenant_id: &str, offering_id: &str) -> Option<ComputeOffering> {
        Self::read_guard(&self.offerings)
            .get(&(tenant_id.to_string(), offering_id.to_string()))
            .cloned()
    }

    async fn upsert_instance(&self, instance: ComputeInstance) {
        let key = (instance.tenant_id.clone(), instance.instance_id.clone());
        Self::write_guard(&self.instances).insert(key, instance);
    }

    async fn get_instance(&self, tenant_id: &str, instance_id: &str) -> Option<ComputeInstance> {
        Self::read_guard(&self.instances)
            .get(&(tenant_id.to_string(), instance_id.to_string()))
            .cloned()
    }

    async fn upsert_allocation(&self, allocation: ComputeAllocation) {
        let key = (
            allocation.tenant_id.clone(),
            allocation.allocation_id.clone(),
        );
        Self::write_guard(&self.allocations).insert(key, allocation);
    }

    async fn get_allocation(
        &self,
        tenant_id: &str,
        allocation_id: &str,
    ) -> Option<ComputeAllocation> {
        Self::read_guard(&self.allocations)
            .get(&(tenant_id.to_string(), allocation_id.to_string()))
            .cloned()
    }

    async fn list_expired_allocations(
        &self,
        tenant_id: &str,
        as_of: &str,
        limit: usize,
    ) -> Vec<ComputeAllocation> {
        let allocations = Self::read_guard(&self.allocations);
        let mut rows: Vec<ComputeAllocation> = allocations
            .iter()
            .filter(|((tenant, _), a)| tenant == tenant_id && a.is_expired(as_of))
            .map(|(_, a)| a.clone())
            .collect();
        rows.sort_by(|a, b| a.lease_expires_at.cmp(&b.lease_expires_at));
        rows.truncate(limit);
        rows
    }
}
