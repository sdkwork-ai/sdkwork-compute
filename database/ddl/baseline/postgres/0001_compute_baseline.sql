-- SDKWork compute consolidated initialization baseline (postgres)
-- Application is in initialization state: full DDL lives here; migrations/ is
-- reserved for post-GA changes.
-- Domain: compute lease and on-demand allocation (REQ-2026-1004 /
-- ADR-20261004). Owner module: sdkwork-compute (runtime platform family;
-- conventions mirror the sandbox module: TEXT composite keys, TEXT enum
-- states with CHECK constraints, optimistic versions, tenant TEXT scope).
-- contract_version: 0.1.0

CREATE TABLE IF NOT EXISTS compute_node (
    tenant_id TEXT NOT NULL,
    compute_node_id TEXT NOT NULL,
    display_name TEXT,
    node_kind TEXT NOT NULL,
    dispatch_endpoint TEXT NOT NULL,
    region TEXT,
    assurance_capabilities JSONB NOT NULL DEFAULT '[]'::JSONB,
    capacity_vcpu INTEGER NOT NULL DEFAULT 0,
    capacity_memory_mb BIGINT NOT NULL DEFAULT 0,
    capacity_disk_mb BIGINT NOT NULL DEFAULT 0,
    max_units INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL,
    last_heartbeat_at TIMESTAMPTZ,
    health_state TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::JSONB,
    version BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMPTZ,
    CONSTRAINT pk_compute_node PRIMARY KEY (tenant_id, compute_node_id),
    CONSTRAINT ck_compute_node_tenant_id CHECK (char_length(tenant_id) BETWEEN 1 AND 128),
    CONSTRAINT ck_compute_node_id CHECK (char_length(compute_node_id) BETWEEN 1 AND 128),
    CONSTRAINT ck_compute_node_kind CHECK (
        node_kind IN ('docker', 'micro_vm_pool', 'bare_metal', 'cloud_pool')
    ),
    CONSTRAINT ck_compute_node_endpoint CHECK (char_length(dispatch_endpoint) BETWEEN 1 AND 512),
    CONSTRAINT ck_compute_node_region CHECK (
        region IS NULL OR char_length(region) BETWEEN 1 AND 64
    ),
    CONSTRAINT ck_compute_node_assurance_capabilities CHECK (
        jsonb_typeof(assurance_capabilities) = 'array'
        AND jsonb_array_length(assurance_capabilities) <= 32
    ),
    CONSTRAINT ck_compute_node_max_units CHECK (max_units BETWEEN 1 AND 100000),
    CONSTRAINT ck_compute_node_status CHECK (
        status IN ('registering', 'active', 'draining', 'disabled')
    ),
    CONSTRAINT ck_compute_node_health_state CHECK (
        health_state IS NULL OR health_state IN ('healthy', 'degraded', 'unhealthy', 'unknown')
    ),
    CONSTRAINT ck_compute_node_version CHECK (version >= 0)
);

COMMENT ON TABLE compute_node IS 'Compute node inventory: dispatch targets that rentable instances run on';
COMMENT ON COLUMN compute_node.tenant_id IS 'Tenant scope; 0 = platform-shared pool';
COMMENT ON COLUMN compute_node.node_kind IS 'Node technology: docker, micro_vm_pool, bare_metal, cloud_pool';
COMMENT ON COLUMN compute_node.dispatch_endpoint IS 'Opaque server-owned dispatch reference; never carries credentials';
COMMENT ON COLUMN compute_node.assurance_capabilities IS 'Isolation levels this node can provide (JSON array of host_user/container/user_space_kernel/micro_vm/dedicated_vm)';
COMMENT ON COLUMN compute_node.max_units IS 'Concurrent rented units the node can host';
COMMENT ON COLUMN compute_node.status IS 'Scheduling lifecycle: registering, active, draining, disabled (draining/disabled host no new allocations)';
COMMENT ON COLUMN compute_node.last_heartbeat_at IS 'Last node liveness heartbeat instant';

-- Scheduler eligibility scan: dispatchable nodes of one kind within a scope,
-- ordered by capacity headroom.
CREATE INDEX IF NOT EXISTS idx_compute_node_eligible
    ON compute_node (tenant_id, node_kind, status, max_units DESC, compute_node_id)
    WHERE deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS compute_offering (
    tenant_id TEXT NOT NULL,
    compute_offering_id TEXT NOT NULL,
    display_name TEXT,
    vcpu INTEGER NOT NULL,
    memory_mb BIGINT NOT NULL,
    disk_mb BIGINT NOT NULL,
    assurance_level TEXT NOT NULL,
    unit_basis TEXT NOT NULL,
    status TEXT NOT NULL,
    version BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMPTZ,
    CONSTRAINT pk_compute_offering PRIMARY KEY (tenant_id, compute_offering_id),
    CONSTRAINT ck_compute_offering_tenant_id CHECK (char_length(tenant_id) BETWEEN 1 AND 128),
    CONSTRAINT ck_compute_offering_id CHECK (char_length(compute_offering_id) BETWEEN 1 AND 128),
    CONSTRAINT ck_compute_offering_vcpu CHECK (vcpu BETWEEN 1 AND 64),
    CONSTRAINT ck_compute_offering_memory_mb CHECK (memory_mb BETWEEN 256 AND 262144),
    CONSTRAINT ck_compute_offering_disk_mb CHECK (disk_mb BETWEEN 1024 AND 1048576),
    CONSTRAINT ck_compute_offering_assurance_level CHECK (
        assurance_level IN ('host_user', 'container', 'user_space_kernel', 'micro_vm', 'dedicated_vm')
    ),
    CONSTRAINT ck_compute_offering_unit_basis CHECK (unit_basis IN ('per_hour', 'per_day')),
    CONSTRAINT ck_compute_offering_status CHECK (status IN ('active', 'retired')),
    CONSTRAINT ck_compute_offering_version CHECK (version >= 0)
);

COMMENT ON TABLE compute_offering IS 'Rentable compute offering (SKU): resource shape plus isolation assurance';
COMMENT ON COLUMN compute_offering.assurance_level IS 'Isolation assurance the offered units provide';
COMMENT ON COLUMN compute_offering.unit_basis IS 'Metering basis for commercial billing hooks';

CREATE TABLE IF NOT EXISTS compute_instance (
    tenant_id TEXT NOT NULL,
    compute_instance_id TEXT NOT NULL,
    owned_by TEXT NOT NULL,
    purpose TEXT NOT NULL,
    compute_offering_id TEXT,
    compute_node_id TEXT,
    vcpu INTEGER NOT NULL,
    memory_mb BIGINT NOT NULL,
    disk_mb BIGINT NOT NULL,
    assurance_level TEXT NOT NULL,
    state TEXT NOT NULL,
    expires_at TIMESTAMPTZ,
    renewed_until TIMESTAMPTZ,
    endpoints JSONB NOT NULL DEFAULT '[]'::JSONB,
    metrics JSONB NOT NULL DEFAULT '{}'::JSONB,
    workspace_ref TEXT,
    last_failure TEXT,
    version BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMPTZ,
    CONSTRAINT pk_compute_instance PRIMARY KEY (tenant_id, compute_instance_id),
    CONSTRAINT ck_compute_instance_tenant_id CHECK (char_length(tenant_id) BETWEEN 1 AND 128),
    CONSTRAINT ck_compute_instance_id CHECK (char_length(compute_instance_id) BETWEEN 1 AND 128),
    CONSTRAINT ck_compute_instance_owned_by CHECK (char_length(owned_by) BETWEEN 1 AND 128),
    CONSTRAINT ck_compute_instance_purpose CHECK (
        purpose IN ('sandbox', 'agent_runtime', 'webserver', 'custom')
    ),
    CONSTRAINT ck_compute_instance_offering CHECK (
        compute_offering_id IS NULL OR char_length(compute_offering_id) BETWEEN 1 AND 128
    ),
    CONSTRAINT ck_compute_instance_node CHECK (
        compute_node_id IS NULL OR char_length(compute_node_id) BETWEEN 1 AND 128
    ),
    CONSTRAINT ck_compute_instance_vcpu CHECK (vcpu BETWEEN 1 AND 64),
    CONSTRAINT ck_compute_instance_memory_mb CHECK (memory_mb BETWEEN 256 AND 262144),
    CONSTRAINT ck_compute_instance_disk_mb CHECK (disk_mb BETWEEN 1024 AND 1048576),
    CONSTRAINT ck_compute_instance_assurance_level CHECK (
        assurance_level IN ('host_user', 'container', 'user_space_kernel', 'micro_vm', 'dedicated_vm')
    ),
    CONSTRAINT ck_compute_instance_state CHECK (
        state IN ('requested', 'provisioning', 'active', 'suspended', 'terminating', 'terminated', 'failed')
    ),
    CONSTRAINT ck_compute_instance_workspace_ref CHECK (
        workspace_ref IS NULL OR char_length(workspace_ref) BETWEEN 1 AND 128
    ),
    CONSTRAINT ck_compute_instance_version CHECK (version >= 0)
);

COMMENT ON TABLE compute_instance IS 'Rented compute instance: one leased unit of compute (sandbox, agent runtime, webserver process, custom)';
COMMENT ON COLUMN compute_instance.owned_by IS 'Subject that rents the instance';
COMMENT ON COLUMN compute_instance.purpose IS 'Product purpose of the rented unit';
COMMENT ON COLUMN compute_instance.compute_node_id IS 'Allocated node once provisioning resolves';
COMMENT ON COLUMN compute_instance.state IS 'Instance lifecycle: requested, provisioning, active, suspended, terminating, terminated, failed';
COMMENT ON COLUMN compute_instance.expires_at IS 'Rental expiry; renewed_until tracks the paid-through instant';
COMMENT ON COLUMN compute_instance.workspace_ref IS 'Opaque consumer-side workspace reference (e.g. sandbox workspace id)';

-- Rental expiry sweep: active instances past expiry.
CREATE INDEX IF NOT EXISTS idx_compute_instance_expiry
    ON compute_instance (tenant_id, state, expires_at, compute_instance_id)
    WHERE deleted_at IS NULL;

-- Owner listing: a subject's rented instances, newest first.
CREATE INDEX IF NOT EXISTS idx_compute_instance_owner
    ON compute_instance (tenant_id, owned_by, created_at DESC, compute_instance_id)
    WHERE deleted_at IS NULL;

-- Node capacity view: instances allocated to one node.
CREATE INDEX IF NOT EXISTS idx_compute_instance_node
    ON compute_instance (tenant_id, compute_node_id, state)
    WHERE deleted_at IS NULL AND compute_node_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS compute_allocation (
    tenant_id TEXT NOT NULL,
    compute_allocation_id TEXT NOT NULL,
    consumer_domain TEXT NOT NULL,
    consumer_ref TEXT NOT NULL,
    compute_instance_id TEXT,
    compute_node_id TEXT,
    requested_target TEXT,
    effective_target TEXT,
    allocation_state TEXT NOT NULL,
    lease_owner TEXT,
    lease_expires_at TIMESTAMPTZ,
    fencing_generation BIGINT NOT NULL DEFAULT 0,
    is_current BOOLEAN NOT NULL DEFAULT TRUE,
    version BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT pk_compute_allocation PRIMARY KEY (tenant_id, compute_allocation_id),
    CONSTRAINT ck_compute_allocation_tenant_id CHECK (char_length(tenant_id) BETWEEN 1 AND 128),
    CONSTRAINT ck_compute_allocation_id CHECK (char_length(compute_allocation_id) BETWEEN 1 AND 128),
    CONSTRAINT ck_compute_allocation_consumer_domain CHECK (
        consumer_domain IN ('agents', 'sandbox', 'webserver', 'custom')
    ),
    CONSTRAINT ck_compute_allocation_consumer_ref CHECK (char_length(consumer_ref) BETWEEN 1 AND 128),
    CONSTRAINT ck_compute_allocation_instance CHECK (
        compute_instance_id IS NULL OR char_length(compute_instance_id) BETWEEN 1 AND 128
    ),
    CONSTRAINT ck_compute_allocation_node CHECK (
        compute_node_id IS NULL OR char_length(compute_node_id) BETWEEN 1 AND 128
    ),
    CONSTRAINT ck_compute_allocation_requested_target CHECK (
        requested_target IS NULL OR requested_target IN ('in_process', 'cloud', 'host')
    ),
    CONSTRAINT ck_compute_allocation_effective_target CHECK (
        effective_target IS NULL OR effective_target IN ('in_process', 'cloud', 'host')
    ),
    CONSTRAINT ck_compute_allocation_state CHECK (
        allocation_state IN ('requested', 'allocating', 'ready', 'active', 'releasing', 'released', 'failed', 'expired')
    ),
    CONSTRAINT ck_compute_allocation_lease CHECK (
        (lease_owner IS NULL AND lease_expires_at IS NULL)
        OR (lease_owner IS NOT NULL AND char_length(lease_owner) BETWEEN 1 AND 128)
    ),
    CONSTRAINT ck_compute_allocation_fencing CHECK (fencing_generation >= 0),
    CONSTRAINT ck_compute_allocation_version CHECK (version >= 0)
);

COMMENT ON TABLE compute_allocation IS 'On-demand allocation: durable binding of a consumer work item to a node/instance with lease and fencing evidence';
COMMENT ON COLUMN compute_allocation.consumer_domain IS 'Owning product domain: agents, sandbox, webserver, custom';
COMMENT ON COLUMN compute_allocation.consumer_ref IS 'Consumer-scoped reference (e.g. agents session placement id)';
COMMENT ON COLUMN compute_allocation.allocation_state IS 'Allocation lifecycle: requested, allocating, ready, active, releasing, released, failed, expired';
COMMENT ON COLUMN compute_allocation.is_current IS 'True for the live allocation of the referenced consumer work item';

-- One live allocation per consumer work item.
CREATE UNIQUE INDEX IF NOT EXISTS uk_compute_allocation_current
    ON compute_allocation (tenant_id, consumer_domain, consumer_ref)
    WHERE is_current = TRUE;

-- Reconciliation sweep: live allocations with lapsed lease evidence.
CREATE INDEX IF NOT EXISTS idx_compute_allocation_reconcile
    ON compute_allocation (allocation_state, lease_expires_at, compute_allocation_id)
    WHERE allocation_state IN ('requested', 'allocating', 'ready', 'active')
        AND lease_expires_at IS NOT NULL;

-- Consumer listing: allocations of one consumer work item, current first.
CREATE INDEX IF NOT EXISTS idx_compute_allocation_consumer
    ON compute_allocation (tenant_id, consumer_domain, consumer_ref, is_current DESC, updated_at DESC);
