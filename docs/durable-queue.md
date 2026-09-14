# Durable CI queue contract

The `queue` slice defines the durable pull boundary used by `gha-indie-worker` workers that may be intermittently online. TypeSpec and JSON Schema are independently authored peer authorities; neither authored file is generated from the other.

## Identity and payload safety

A queue job is bound to an immutable `repository` + 40-character Git revision + `workflow_path` + reviewed `profile` + `attempt` + `salt`. Queue payloads deliberately contain capability requirements and scheduling metadata only. Secret values, arbitrary environment values, caller-selected shell commands, and mutable image tags do not belong in this contract.

All new wire fields in this slice use `snake_case`.

## Lifecycle

The durable lifecycle is `queued -> claimed -> running -> succeeded|failed`, with `cancelled` and `expired` terminal escape states. A claimed job may be returned to `queued` only through an explicit lease-reclaim path. Reclaim preserves the original `enqueued_at`, increments the attempt/generation authority used by the implementation, and must never authorize a stale worker to publish a terminal result.

## Lease and fencing

Each claim creates a `JobLease` with a monotonically increasing `generation` and `fencing_token`. Heartbeats renew the lease before `expires_at`. Completion, cancellation acknowledgement, artifact publication, and check-run publication must compare against the current lease generation/fence; stale workers fail closed.

`ClaimReceipt` is immutable eligibility evidence. It records matched OS, architecture, runtime, profile, features, trust tier, and worker concurrency state, but no credentials.

## Scheduling

Eligibility is evaluated before ordering: OS, architecture, profile, optional runtime, required features, minimum trust tier, and available worker capacity must all match. A worker at `max_concurrency` is ineligible to claim.

Among eligible jobs, implementations should order by effective priority with a bounded age boost, then `available_at`, then stable job ID. The exact age formula belongs in the implementation, but it must be deterministic and bounded so an old low-priority job eventually progresses without permanently overriding explicit priority.

## Supersession

A newer immutable PR head may supersede an older queued or in-flight job. `JobSupersession` links the stale job to its replacement, and the stale job is cancelled with reason `superseded`. A stale lease remains fenced even if its process continues running.

## Validation

`contracts/config/queue.config.json` participates in the normal `ores-contracts` and TJSV sweeps. Fixtures under `contracts/fixtures/queue` exercise canonical valid jobs, supersession, SHA validation, lifecycle vocabulary, priority bounds, required profiles, trust tiers, and rejection of undeclared payload fields.
