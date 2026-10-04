//! Domain types for the compute lease and on-demand allocation domain.
//! Enum values mirror the PostgreSQL CHECK constraints in the baseline DDL.

use serde::{Deserialize, Serialize};

// ---------- Node ----------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeNode {
    pub tenant_id: String,
    pub node_id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    pub node_kind: String,
    pub dispatch_endpoint: String,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub assurance_capabilities: Vec<String>,
    #[serde(default)]
    pub capacity_vcpu: i32,
    #[serde(default)]
    pub capacity_memory_mb: i64,
    #[serde(default)]
    pub capacity_disk_mb: i64,
    #[serde(default = "default_max_units")]
    pub max_units: i32,
    pub status: String,
    #[serde(default)]
    pub last_heartbeat_at: Option<String>,
    #[serde(default)]
    pub health_state: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub deleted_at: Option<String>,
}

fn default_max_units() -> i32 {
    1
}

impl ComputeNode {
    pub fn is_scheduler_eligible(&self) -> bool {
        self.deleted_at.is_none() && self.status == "active"
    }
}

// ---------- Offering ----------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeOffering {
    pub tenant_id: String,
    pub offering_id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    pub vcpu: i32,
    pub memory_mb: i64,
    pub disk_mb: i64,
    pub assurance_level: String,
    pub unit_basis: String,
    pub status: String,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub deleted_at: Option<String>,
}

// ---------- Instance ----------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeInstance {
    pub tenant_id: String,
    pub instance_id: String,
    pub owned_by: String,
    pub purpose: String,
    #[serde(default)]
    pub offering_id: Option<String>,
    #[serde(default)]
    pub node_id: Option<String>,
    pub vcpu: i32,
    pub memory_mb: i64,
    pub disk_mb: i64,
    pub assurance_level: String,
    pub state: String,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub renewed_until: Option<String>,
    #[serde(default)]
    pub endpoints: Vec<serde_json::Value>,
    #[serde(default)]
    pub metrics: serde_json::Value,
    #[serde(default)]
    pub workspace_ref: Option<String>,
    #[serde(default)]
    pub last_failure: Option<String>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub deleted_at: Option<String>,
}

impl ComputeInstance {
    pub fn is_active(&self) -> bool {
        self.deleted_at.is_none() && self.state == "active"
    }
}

// ---------- Allocation ----------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeAllocation {
    pub tenant_id: String,
    pub allocation_id: String,
    pub consumer_domain: String,
    pub consumer_ref: String,
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub node_id: Option<String>,
    #[serde(default)]
    pub requested_target: Option<String>,
    pub effective_target: String,
    pub allocation_state: String,
    #[serde(default)]
    pub lease_owner: Option<String>,
    #[serde(default)]
    pub lease_expires_at: Option<String>,
    #[serde(default)]
    pub fencing_generation: i64,
    pub is_current: bool,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl ComputeAllocation {
    pub fn occupies_capacity(&self) -> bool {
        matches!(
            self.allocation_state.as_str(),
            "requested" | "allocating" | "ready" | "active"
        )
    }

    pub fn is_expired(&self, as_of: &str) -> bool {
        self.occupies_capacity()
            && self
                .lease_expires_at
                .as_deref()
                .is_some_and(|expires| expires < as_of)
    }
}

// ---------- Vocabulary re-exports (compile-time constants for consumers) ----------

pub mod vocabulary {
    pub const NODE_KIND_DOCKER: &str = "docker";
    pub const NODE_KIND_MICRO_VM_POOL: &str = "micro_vm_pool";
    pub const NODE_KIND_BARE_METAL: &str = "bare_metal";
    pub const NODE_KIND_CLOUD_POOL: &str = "cloud_pool";

    pub const NODE_STATUS_REGISTERING: &str = "registering";
    pub const NODE_STATUS_ACTIVE: &str = "active";
    pub const NODE_STATUS_DRAINING: &str = "draining";
    pub const NODE_STATUS_DISABLED: &str = "disabled";

    pub const INSTANCE_STATE_REQUESTED: &str = "requested";
    pub const INSTANCE_STATE_PROVISIONING: &str = "provisioning";
    pub const INSTANCE_STATE_ACTIVE: &str = "active";
    pub const INSTANCE_STATE_SUSPENDED: &str = "suspended";
    pub const INSTANCE_STATE_TERMINATING: &str = "terminating";
    pub const INSTANCE_STATE_TERMINATED: &str = "terminated";
    pub const INSTANCE_STATE_FAILED: &str = "failed";

    pub const ALLOCATION_STATE_REQUESTED: &str = "requested";
    pub const ALLOCATION_STATE_ALLOCATING: &str = "allocating";
    pub const ALLOCATION_STATE_READY: &str = "ready";
    pub const ALLOCATION_STATE_ACTIVE: &str = "active";
    pub const ALLOCATION_STATE_RELEASING: &str = "releasing";
    pub const ALLOCATION_STATE_RELEASED: &str = "released";
    pub const ALLOCATION_STATE_FAILED: &str = "failed";
    pub const ALLOCATION_STATE_EXPIRED: &str = "expired";

    pub const TARGET_IN_PROCESS: &str = "in_process";
    pub const TARGET_CLOUD: &str = "cloud";
    pub const TARGET_HOST: &str = "host";
}
