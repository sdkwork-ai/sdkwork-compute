# SDKWork Compute Database

Owner: `sdkwork-runtime-platform`

Canonical contract: `database/contract/schema.yaml` (`0.1.0`)

Physical authority: `database/ddl/baseline/postgres/0001_compute_baseline.sql`

## Scope

Compute lease and on-demand allocation domain (REQ-2026-1004 /
ADR-20261004):

```text
compute_node (inventory) -- hosts compute_instance (rented units)
compute_offering (SKU)        |
compute_allocation (consumer binding: agents / sandbox / webserver)
```

Consumers (agents, sandbox, webserver) persist only allocation/instance
references; node and instance state lives here.

## Engine And Lifecycle

PostgreSQL only. `baseline-plus-migrations`: the `0.1.0` baseline holds the
complete 4-table schema; `migrations/` is reserved for post-GA changes.
`autoMigrate` is disabled; release workflows run `db:migrate` explicitly.

## Verification

```bash
pnpm db:validate
pnpm db:materialize:contract
pnpm db:plan
```
