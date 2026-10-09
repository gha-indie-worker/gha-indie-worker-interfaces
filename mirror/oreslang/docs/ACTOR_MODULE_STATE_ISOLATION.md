# Actor module-state isolation (draft PR #519)

## Representation and invariant

A module import resolves to an immutable executable declaration/code image, NOT
a mutable process singleton. A module with writable fields has two conceptual
components:

- `ModuleCode<CodeUnit, Module, Generation>`: cached AST, functions, read-only
  metadata. Shared by actors within one OresContext/code generation.
- `ModuleState<Owner, CodeUnit, Module, Generation>`: runtime fields. Owned by a
  single logical execution process. Mutable references must never cross owners.

Current JVM implementation stores module fields in an actor's `ActorCell` when
`ActorRuntime.currentActorId()` is present. The actor runtime validates its
own turn's execution owner before returning the state. Fields are lazily
initialized exactly once, and state is reclaimed on actor retirement. Import
targets use the *callee source-unit identity* as part of their state key, so
different actors importing the same module never alias its writable slots.

ROOT has a separate per-evaluator map for its own serial execution domain.
Unadmitted host threads, unrelated runtimes and ActorGroup mailmen **fail
closed** for module-field access. Supervisors now have independent
generation-qualified module cells keyed to their admitted single-flight
scheduler lanes, automatically discarded at supervisor retirement.
Do not silently route other processes' accesses to ROOT.

This is logical isolation inside one JVM, not physical heap separation, and
does not establish native JIT code-cache sharing across Truffle Engines.

## Initialization and writes

- First access initializes each field in the actor/ROOT owner's turn; later
  reads return the stored value without re-executing its initializer.
- Recursive initialization is rejected rather than exposing partially
  initialized state.
- Only a public `let` field may be reassigned by a module member write.
  `val`/`const` are not rebindable.
- The compiler/ownership pass validates local module writes. Cross-code-unit direct reads/calls/writes are checked by the linker against
  explicit imported module declarations, visibility, callable arity and
  writable-let permissions. Generic substitutions and wildcard namespace
  projections still require full common typed Contract IR; runtime validation
  is defense in depth.
- Source module `init` runs in ROOT during linked-program bootstrap. It must
  not be treated as actor-local mutable module initialization.

## Actor messages and Scala comparison

Scala/Akka's sequential mailbox execution does not give a borrow checker or
deep immutability guarantee. JVM garbage collection is memory management,
not race prevention; Akka application messages are typically immutable by
convention. Oreslang instead requires safe message publication:

- An ordinary actor delivery freezes/copies the supported data graph, with
  unsafe host objects, executable capabilities and cycles rejected.
- An explicit `Shared<T>` handle reuses a deeply frozen snapshot across
  distinct SHARED actors, without exposing a writable alias.
- `rt copy` creates new owned mutable storage for supported graphs.
- `rt share` of arbitrary move-only mutable data still fails closed: a typed
  readonly projection, nested shape checking, and escape prevention are not
  proven complete yet.
- Every actor owns at most one active guest turn, while different actor lanes
  can run concurrently. Mailbox publication uses JVM happens-before edges,
  not a requirement to manually flush CPU caches.

## Remaining gates

1. Run **executed** exact-head JVM compiler, imported-module and mailbox tests.
   A runner startup failure is not a test failure or pass.
2. Add full source-level cross-unit module field contracts, supervisor-owned
   module scopes, and hot-reload generation fencing for old module references.
3. Enforce typed deep-readonly `rt share` through classes, Option/Result,
   collections, callback captures and host interop.
4. Exercise module state on actor startup, await/resume, cancellation, restart,
   cyclic import and concurrent imported-module calls.

Keep PR #519 a draft. The Rust bounded formal model concerns an abstract
scheduler/publication protocol; it does not prove Java module-state safety.
