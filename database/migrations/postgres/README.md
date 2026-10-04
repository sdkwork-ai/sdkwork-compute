# PostgreSQL Migrations

Authoritative Compute migrations use sortable `*.up.sql` files with explicit
SDKWork migration metadata. The application is pre-launch: the consolidated
baseline holds the full schema and the ordered migration set is empty.
Production rollback uses compatible application rollback or reviewed
forward-fix cutover; migrations are not automatically reversed.

SQLite migrations are forbidden in this authoritative-server root.
