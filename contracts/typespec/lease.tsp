// ============================================================================
// TypeSpec authority - the v1 lease protocol this repository already publishes.
//
// PEER of contracts/json-schema/lease.schema.json.
//
// These three types are not new. `WorkerLease` and `Health` are already in
// `src/protocol.rs`, `InterfaceError` is already in `src/error.rs`, and
// `schema/v1/workerlease.json` is the published JSON Schema for the lease -- a
// single description with no peer, which is exactly the situation the rest of
// this directory exists to fix. So they are described here too, and
// scripts/parity.mjs additionally checks that the published v1 document and the
// $defs peer of it still agree: the v1 artifact keeps its URL and its shape, and
// gains a second authority rather than being replaced by one.
//
// Bounds are copied from the published document deliberately. `id` and `revision`
// have `minLength: 1` and NO maximum there, and inventing a maximum here would
// tighten a contract that consumers are already coded against.
// ============================================================================
namespace GhaIndieWorker;

@doc("Why a lease was refused. `InterfaceError` in src/error.rs; the crate parses and validates, so these are the only three ways parsing fails.")
enum LeaseErrorCode {
  @doc("`id` was empty or whitespace.")
  EmptyId: "empty_id",

  @doc("`revision` was empty or whitespace.")
  EmptyRevision: "empty_revision",

  @doc("`payload` does not match the schema `revision` names.")
  SchemaMismatch: "schema_mismatch",
}

@doc("A worker's claim on a unit of work. `protocol::WorkerLease`; published as schema/v1/workerlease.json.")
model WorkerLease {
  @doc("Lease identity. Non-empty; deliberately unbounded above, as the published v1 schema has it.")
  @minLength(1)
  id: string;

  @doc("Schema revision the payload conforms to, e.g. `gha-indie-worker-0001`. A consumer that does not know the revision refuses the lease rather than guessing.")
  @minLength(1)
  revision: string;

  @doc("Lease body, shaped by `revision`. Free-form here on purpose: this envelope is versioned by `revision`, and typing the payload in the envelope would make every payload change an envelope change.")
  payload: Record<unknown>;
}

@doc("Liveness answer. `protocol::Health`.")
model Health {
  @doc("False only when the service is up enough to answer and still unable to serve.")
  ok: boolean;

  @minLength(1) @maxLength(64) service: string;

  @doc("Wire protocol version as a string. `protocol::PROTOCOL_VERSION`.")
  @minLength(1)
  @maxLength(16)
  protocol: string;
}
