# Contract execution hardening — 2026-09-17

This follow-up closes a false-positive execution boundary discovered while finishing the durable pull-queue contract PR. The slice wrapper could report success when `npx ores-contracts` exited zero without running the installed scoped package CLI, leaving neither generated projections nor `target/ores-contracts` receipts. The Rust witness was the first lane that proved the outputs were absent.

## Fifteen-task ledger

1. Invoke the installed `@oresoftware/ores-contracts` CLI module explicitly instead of relying on ambiguous `npx ores-contracts` resolution.
2. Fail closed when the installed scoped CLI module is absent.
3. Treat child-process spawn errors as slice failures.
4. Treat signal termination as a slice failure.
5. Treat every nonzero CLI exit status as a slice failure.
6. Remove a stale target receipt before each check/generate run so old evidence cannot satisfy a new run.
7. Require every check/generate invocation to recreate its configured target receipt.
8. Require the recreated receipt status to be exactly `passed`.
9. Before generate, remove each configured output artifact that the invocation must recreate.
10. Require every configured generated artifact to exist after generate returns zero.
11. Require the generated output receipt to exist after generate.
12. Resolve each slice output and target relative to that slice's config file rather than assuming a global directory convention.
13. Make the Rust witness verify its three required projections (`rust/types.rs`, `seaorm/entities.rs`, `diesel/schema.rs`) before compiling.
14. Invoke the installed scoped CLI directly for the PostgreSQL DB witness so a zero-op npm-bin shim cannot create false DB parity evidence.
15. Rename the parity workflow generation step so it claims only what it proves; a tracked-diff check is not used as evidence that untracked generated outputs are current.

## Evidence policy

A slice is successful only when the intended installed CLI process executes and command-specific evidence is recreated during that invocation. A zero exit code without receipts/artifacts is a failure. Rust and PostgreSQL witnesses remain independent downstream checks rather than substitutes for projection parity.

GitHub Actions jobs that never execute steps remain runner-admission non-evidence. Any stepful failure remains a merge blocker until fixed.
