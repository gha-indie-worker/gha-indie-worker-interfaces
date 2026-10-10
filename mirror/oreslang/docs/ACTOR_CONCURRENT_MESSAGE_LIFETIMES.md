# Concurrent actor-message lifetimes, serialized guest execution

Status: DRAFT (PR #519); exact-head executed JVM verification required before merge.

## Model

Actor concurrency is *maximum admitted unfinished receives*. It is **not**
execution parallelism. Each SHARED, PRIVATE and UNTRUSTED actor has exactly one
**native-thread execution lease** (an actor-local atomic CAS lock backed by
`guestTurnOwner`). Five messages may have their own in-flight stackless
continuations, each waiting for I/O or channels, but at most **one OS/native
carrier thread may execute ANY actor code at any instant**. This includes
compiled Oreslang, native/runtime-backed behavior callbacks, startup, on_start,
resumed continuations, actor-local memory access and final guest cleanup.

The native lease is acquired BEFORE installing actor ThreadLocal execution
authority and released at turn completion. Each physical carrier may execute
other actors once it has released the lease; it is **never pinned to an actor
for the duration of an await**. A resumed handler may acquire the same actor
lease on another carrier. The native Thread is the lease owner, not the
message UUID/sequence. The ActorGroup Mailman has a separate CONTROL execution
lease and may enqueue another actor's immutable mail but never call that
actor's code directly.

Supervisor stop/cancel must revoke future admission immediately, but cannot
clear a live actor's active message payload/fields/owned memory from a foreign
thread. Those resources retire after the executing native turn exits and the
TurnExecutor/Truffle carrier boundary returns.

```oreslang
define actor Worker as
  int concurrency: 5;  // Immutable declaration metadata; default is 1.

  receive(ActorMail<String> m): void {
    m.send("progress");  // One output correlated with m.
    // await/rt cooperate yield the actor lane without ending this message.
    m.done();             // Complete m, not the actor.
    return;
  }
end
```

`m.send()` sends `m.value`; `m.send(value)` sends an explicit output.
`self.send(value)` remains a general actor output, and automatically carries
the current message sequence when invoked inside a receive. ActorOutput
exposes `message_sequence: Option<int>`. ActorGroup Mailman `ActorMail`
now exposes `requestSequence: Optional<long>` for the same correlation.
`m.message_id` is an opaque
`ActorId(UUID):sequence` string. It is not a shared pointer, send authority,
or memory address.

Messages default to auto-complete at the end of a successful synchronous or
stackless receive, avoiding leaked admission slots. `m.done()` explicitly
releases the slot sooner and prohibits subsequent `m.send` or `self.send`
for that same message. A queued continuation after explicit completion finishes
quietly without resurrecting the message.

## Admission / fairness / quotas

- `concurrency` must be compile-time immutable in 1..1024 (1..32 untrusted);
  it cannot be changed through a runtime actor field write. The runtime
  seals configuration permanently at first user-mail admission, even when
  that first receive has already completed; later host callbacks cannot
  reconfigure it after the active-message table becomes empty.
- Ordinary actor mail admission pauses when active receives reach the limit.
  Control continuations have priority, but after 8 continuations one eligible
  user message may enter to prevent `rt cooperate` starvation.
- The existing actor single-flight lease still forbids two simultaneous
  guest turns. `await` yields and continuation execution runs through the
  owning actor's CONTROL continuation mailbox.
- The actor mailbox transport reserves memory **through the entire admitted
  unfinished message lifetime**, including suspended source handlers, until
  implicit/explicit completion or termination. Releasing on first yield would
  hide potentially large retained message graphs from quota accounting.
- Private and untrusted actor inputs use isolated copies. SHARED inputs use
  deep immutable transport snapshots; explicit core `Shared<T>` uses a typed
  frozen reference. Untrusted code cannot smuggle a live actor/shared pointer
  into its inbox. Correlation metadata is pure value data.
- `self.end()` and structured cancellation close the actor mailbox, cancel
  outstanding futures, release active message leases and retire actor-owned
  state/resource scopes. `m.done()` never stops the actor.

## Important boundary cases

The actor-output sequence counts outputs; message sequence counts incoming
messages. These counters differ when one incoming message produces multiple
outputs. Never infer request identity from output sequence alone.

A message may be waiting while a later message modifies the same actor's
private state. No data race exists because guest execution is serialized,
but **logical interleaving still matters**. For multi-step transactions,
revalidate invariants after `await`, use a state machine, or reserve an
explicit actor-local logical transaction token. Absence of data races does
not imply serializable transactions.

Untrusted actors retain per-turn fuel/cancellation checks plus a reduced
message admission bound. Per-message aggregate fuel and retained resource
budgets should receive formal and adversarial testing before enabling larger
limits.

## Verification

Run `ActorNativeCarrierLeaseTest`,
`ActorMailboxExecutionBoundaryTest`,
`ActorCarrierIdentityCleanupTest`,
`ActorConcurrentMailboxExecutionTest`, the pre-existing
`SourceActorSpawnExecutionTest` default-concurrency ordering regression,
`ActorCooperateOrderingTest`, `ActorSharedMailboxAccountingTest`,
`ActorContinuationCapacityTest`, `ActorStructuredCancellationTest`,
`ActorModuleStateOwnershipTest` and the Rust scheduler/publication models, including
`formal/process-turns/tests/concurrent_mail.rs`'s exhaustive 1/2-inflight
reference model and explicit mutation-oracle counterexamples.

Must test: 1/5/32/1024 limits; 5 suspended handlers; early done;
double done; output correlation; priority fairness under hot cooperate loops;
actor termination while suspended; memory reservation exact release; untrusted
input copies; recursive/opaque hostile payloads; AOT/code-generation fencing.

As of this draft update the **Maven JVM suite has not executed** due runner
startup/admission failures. Static source changes are not a passing build.
