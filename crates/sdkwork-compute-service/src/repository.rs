//! In-memory compute repository for testing and lightweight deployments.

use std::collections::HashMap;
use std::sync::RwLock;

use crate::model::{ComputeAllocation, ComputeInstance, ComputeNode, ComputeOffering};

#[derive(Debug, Default)]
pub struct InMemoryComputeRepository {
    nodes: RwLock<HashMap<(String, String), ComputeNode>>,
    offerings: RwLock<HashMap<(String, String), ComputeOffering>>,
    instances: RwLock<HashMap<(String, String), ComputeInstance>>,
    allocations: RwLock<HashMap<(String, String), ComputeAllocation>>,
}

impl InMemoryComputeRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert_node(&self, node: ComputeNode) {
        let key = (node.tenant_id.clone(), node.node_id.clone());
        self.nodes.write().unwrap().insert(key, node);
    }

    pub fn get_node(&self, tenant_id: &str, node_id: &str) -> Option<ComputeNode> {
        self.nodes
            .read()
            .unwrap()
            .get(&(tenant_id.to_string(), node_id.to_string()))
            .cloned()
    }

    pub fn list_eligible_nodes(
        &self,
        tenant_id: &str,
        node_kind: Option<&str>,
    ) -> Vec<ComputeNode> {
        let nodes = self.nodes.read().unwrap();
        let mut rows: Vec<ComputeNode> = nodes
            .iter()
            .filter(|((t, _), n)| {
                t == tenant_id
                    && n.is_scheduler_eligible()
                    && node_kind.is_none_or(|k| n.node_kind == k)
            })
            .map(|(_, n)| n.clone())
            .collect();
        rows.sort_by(|a, b| b.max_units.cmp(&a.max_units));
        rows
    }

    pub fn upsert_instance(&self, instance: ComputeInstance) {
        let key = (instance.tenant_id.clone(), instance.instance_id.clone());
        self.instances.write().unwrap().insert(key, instance);
    }

    pub fn get_instance(&self, tenant_id: &str, instance_id: &str) -> Option<ComputeInstance> {
        self.instances
            .read()
            .unwrap()
            .get(&(tenant_id.to_string(), instance_id.to_string()))
            .cloned()
    }

    pub fn upsert_allocation(&self, allocation: ComputeAllocation) {
        let key = (
            allocation.tenant_id.clone(),
            allocation.allocation_id.clone(),
        );
        self.allocations.write().unwrap().insert(key, allocation);
    }

    pub fn get_allocation(
        &self,
        tenant_id: &str,
        allocation_id: &str,
    ) -> Option<ComputeAllocation> {
        self.allocations
            .read()
            .unwrap()
            .get(&(tenant_id.to_string(), allocation_id.to_string()))
            .cloned()
    }

    pub fn list_expired_allocations(&self, as_of: &str, limit: usize) -> Vec<ComputeAllocation> {
        let allocations = self.allocations.read().unwrap();
        let mut rows: Vec<ComputeAllocation> = allocations
            .iter()
            .filter(|(_, a)| a.is_expired(as_of))
            .map(|(_, a)| a.clone())
            .collect();
        rows.sort_by(|a, b| a.lease_expires_at.cmp(&b.lease_expires_at));
        rows.truncate(limit);
        rows
    }
}
