# Oreslang ownership, immutable sharing, and automatic resource reclamation

Status: draft PR #519. This is an in-progress compiler/runtime contract;
**do not merge** until an exact-head JVM suite actually executes.

## Ownership-first, not a JVM-free tracing collector

Rust's ordinary heap values have **ownership, affine moves, borrows and
destructors**; Rust does not use a conventional tracing garbage collector by
default. Oreslang can target the same developer experience of almost never
calling `gc()`, but **99% is a product goal, not a measured guarantee**.

- `rt take`: moves ownership; compiler invalidates the old binding.
- `rt borrow`: read-only, lifetime-bounded, non-escaping view.
- `rt borrow mut`: exclusive mutable borrow.
- `rt copy`: independently allocates an owned copy of the supported acyclic
  data graph; unsupported host/capability objects fail closed.
- `rt share`: constructs a detached deeply immutable snapshot of a proven
  owned graph and propagates a `Readonly<T>` view through nested accesses.
  Scalars are copied/reused as immutable values.
- Actor/supervisor/ROOT process scope retirement releases registered external
  resources **automatically**. The JVM collector subsequently reclaims
  unreachable ordinary heap objects. A periodical weak-handle sweep remains a
  safety net for abandoned host/interop handles.

No userland mutex is necessary for ordinary actor-local state. Shared mutable
capabilities remain explicit, core-gated and synchronized.

## Readonly publication guarantees, current implementation

Compiler: OwnershipChecker rejects `rt share` unless the source is a known
data-only deep-copyable type. TypeChecker returns `Readonly<T>` for non-scalar
values; direct writes, mutable-list methods, readonly object method calls,
forged casts and `new Readonly<T>()` are disallowed. Nested field/index
projections preserve readonly capability.

Runtime: `rt share` performs the already-bounded `rt copy` walk followed
by recursive freezing into unmodifiable collections and readonly nominal
object snapshots. It rejects cycles, opaque host objects, actor handles and
callable capabilities; readonly nominal object snapshots cannot run instance
methods or accept field writes. Future work includes a trusted `Shared<T>`
publication token for repeated zero-copy actor deliveries; **this pass does
not claim zero-copy sends from `rt share` alone**.

Remaining checks: dynamic reflection/interop escape routes, typed generics,
closures capturing readonly or writable objects, Option/Result method
projections, maps/sets with object keys, deep cyclic/shared-graph handling,
AOT backends and mutable alias escape through casts not routed through
TypeChecker. Unknown and unsupported capabilities must continue failing closed.

## Imported module contracts

`LinkedModuleContractChecker` resolves explicit imported modules against
the complete source-closure graph and visits function/class bodies without
reflection. Direct `M.field`, `M.call(...)` and `M.field = value` check
public exports, arity and mutable-`let` permissions. It rejects computed
namespace indexing. The evaluator repeats access checks dynamically.

This does **not** yet replace full cross-unit type inference: type arguments,
overloads with generic substitutions, wildcard namespace projections, imported
function-value escapes and runtime host signatures still need common typed
Contract IR/TypeSpec-compatible validation before a global soundness claim.

## Process-local module storage and hot-reload fencing

The immutable CodeImage gets a monotonically allocated VM-local generation
ID. Within one `OresContext`, a code unit cannot be silently replaced by
another AST. Its generation is part of actor/supervisor mutable module-state
identity. Different actors and supervisors have different slots, even when
using identical shared code. Each supervisor's slots and registered resource
cleanups retire when its scheduler closes; actor cleanup occurs during actor
finalization and ROOT cleanup after main completion.

A new version must link under a different generation-qualified code-unit
identity or a newly built context. Safe atomic activation, old-generation
continuation fencing, versioned import switching and explicit state-migration
contracts are **not yet implemented**; do not claim transparent hot reload.

## Verification gates

- JVM compilation and execution of the focused Maven suite at the exact
  source/mirror Git blob SHAs.
- Static negative tests (readonly mutation, cast forgery, module non-public
  export, invalid call arity, immutable imported field).
- Parallel actor/supervisor module-state alias checks and retirement tests.
- Java runtime refinement tests proving no writable alias after share, before
  declaring full Send/Sync-like safety.
- Rust state-machine model is complementary bounded abstract evidence only,
  not Java/Truffle proof.
