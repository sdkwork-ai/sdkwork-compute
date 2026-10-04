# SDKWork Compute Module

Compute lease and on-demand allocation domain (REQ-2026-1004 /
ADR-20261004). Single owner of compute nodes, rentable offerings/instances
and consumer allocations. Consumers (agents, sandbox, webserver) persist
only allocation/instance references and integrate through the compute API
and SDK.

## Spec Resolution

1. Read this AGENTS.md.
2. Database authority: `database/contract/schema.yaml` (`0.1.0`); physical
   authority `database/ddl/baseline/postgres/0001_compute_baseline.sql`
   (baseline-plus-migrations, pre-launch consolidation).
3. Global standards: `../sdkwork-specs/README.md`, then task-specific specs
   (`DATABASE_SPEC.md`, `DATABASE_FRAMEWORK_SPEC.md`, `RUST_CODE_SPEC.md`,
   `API_SPEC.md` as applicable).
4. Design: `../../sdkwork-agents/docs/architecture/decisions/ADR-20261004-compute-lease-domain.md`.

## Conventions

Runtime-platform family (mirror `sdkwork-sandbox`): TEXT composite keys
(`tenant_id` 1..128, `0` = platform-shared pool), TEXT enum states with
CHECK constraints, optimistic `version`, `TIMESTAMPTZ DEFAULT
CURRENT_TIMESTAMP`, soft delete via `deleted_at`.

## Verification

```bash
pnpm db:validate
pnpm db:materialize:contract
pnpm db:plan
```

## Boundaries

- The kernel owns the runtime SPI inside instances; compute owns the
  environment (nodes, offerings, instances, allocations).
- Consumers must not duplicate node/instance state; they persist
  allocation/instance references only.
- Phase status: database domain scaffolded; service/API/worker crates are
  the next increments (ADR roadmap phases 1-2).
