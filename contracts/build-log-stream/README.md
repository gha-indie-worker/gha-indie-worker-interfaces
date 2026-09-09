# GitHub Actions build-log stream contract

Tracking: DEN-3440. This is a wire contract only. Database tables, Loki labels,
object-store bucket layout, and provider credentials are runtime/storage policy and
must not leak into the cross-language envelope.

## Why this exists

GitHub's documented Actions REST API supports downloading job/run logs, but does
not expose a supported byte-by-byte live-tail API. For runners we control, live
stdout/stderr therefore originates at the executor boundary. Signed GitHub
`workflow_run` / `workflow_job` lifecycle events identify and reconcile the run,
and the documented terminal log download is used to detect gaps and backfill final
evidence.

For GitHub-hosted runners that we do not control, the supported baseline is
lifecycle events plus terminal log download. An opt-in workflow wrapper may stream
a particular command's output, but it is not a transparent replacement for
runner-level capture and must not be described as one.

## Recommended storage plane

1. **Hot stream:** emit validated/redacted `BuildLogRecord` envelopes through an
   OTLP collector to Loki/Grafana (or another OTLP-compatible logs backend).
2. **Durable raw evidence:** write bounded compressed chunks to org-isolated
   object storage and retain a `BuildLogChunkManifest` with a SHA-256 digest.
3. **Supabase/Postgres:** store run/job/step metadata, chunk manifests, gap state,
   retention policy, tenant/org/repo indexes, and pointers to hot/raw storage.
   Postgres is not the primary unbounded stdout/stderr blob store.
4. **Terminal reconciliation:** download the documented GitHub job/run logs after
   completion, compare coverage/digests, record `BuildLogGap` and
   `BuildLogTerminalReconciliation`, and backfill missing final evidence.

## Required identity and ordering

Every record binds GitHub organization, repository, workflow, run ID, run attempt,
job ID, exact head SHA, and ref. `sequence` is monotonically increasing per
`(organization, repository, runId, runAttempt, jobId, stream)` and `offsetBytes`
tracks the source byte offset. A runtime idempotency key should include that stream
identity plus sequence; durable chunks additionally bind their SHA-256 digest.

## Security and backpressure

- External payloads are validated before persistence or fan-out.
- Runner emitters redact/mask before network emission and never attach arbitrary
  environment variables, authorization headers, cookies, or credentials as log
  attributes.
- Ingest queues are bounded. Collector/object-store failure must spool or drop
  according to explicit policy; it must never deadlock the build indefinitely.
- Duplicate records are idempotent. Out-of-order records create gap evidence
  instead of silently rewriting sequence history.
- Runtime credentials are short-lived and scoped to the owning org/repo ingest
  namespace. No secret value belongs in this repository or contract fixtures.

## Authority rule

`main.tsp` and `authored.schema.json` are independently maintained peer
authorities. Neither is generated from the other. The JSON Schema emitted from
TypeSpec by `ORESoftware/typespec-json-schema-validator` is comparison evidence
only. CI fails closed on unexplained structural or behavioral disagreement and
also contains an intentional-drift negative control proving the gate can fail.
