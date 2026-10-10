# ActorGroup Event Bus

This document defines the first OresVM runtime contract for tightly coupled actor
cooperation. It is intentionally layered on top of the actor/channel runtime:
mailboxes remain the default actor-to-actor transport, while an ActorGroup event
bus is an opt-in hot path for solver/search/graph workloads that exchange useful
state repeatedly.

## Separation of responsibilities

The supervision tree and ActorGroup membership are orthogonal.

- The supervision tree answers who owns, cancels, restarts, and observes failure.
- An ActorGroup answers which actors are cooperating on one problem.
- A Mailbox is the private inbox of one actor.
- A Channel is the bounded queue and atomic select/waiter primitive. Each event
  subscription owns a channel; Mailman uses a bounded channel as its outbox.
- An ActorEventBus is runtime infrastructure owned by an ActorGroup. It is not
  an actor and has no central mailbox/dispatcher.

An actor may therefore cooperate with cousins, nephews, or other descendants
without changing structured ownership. Membership requires an unforgeable,
runtime-affine ActorGroupJoinCapability that may be explicitly forwarded through
actor messages. Rotating the capability prevents new joins without ejecting
existing members.

Actor-created groups close automatically when their creator terminates.
Host-created groups live until explicitly closed or until ActorRuntime teardown.
Actor termination or group leave closes that actor's subscriptions, fails pending
reads/writes, and releases unread payload reservations before membership is forgotten.
Group close drains subscriptions and the Mailman outbox. A Mailman callback already
in progress may finish, but its context cannot send after group close.

## Mailbox and Mailman routing

Use ActorRef.send for addressed commands and replies. It enters the target actor's
private mailbox and preserves that actor's transport/isolation policy. Publishing
an event fans out to current subscriptions; it does not invoke actor behavior or
implicitly enqueue a mailbox message. Actors read subscription Futures explicitly.
There is no total order between mailbox arrivals and event arrivals.

An optional Mailman consumes group.emit output on CONTROL carriers in bounded
quanta, one callback at a time. Its FIFO outbox uses ChannelRuntime.tryWrite and
tryRead; a full outbox rejects immediately without registering a pending writer.
Each envelope is frozen once, holds one aggregate-memory reservation, and releases
it on callback completion, admission rejection, dispatch failure, or teardown.

ActorGroupContext.send routes through the ordinary target mailbox, including its
capacity and isolation checks. Holding an ActorRef permits routing outside the
group; membership is not a replacement for the reference capability. Every context
operation requires its owning callback and carrier. Capturing a context, calling it
from another thread, or using it in a later callback fails with SecurityException.
Mailman callback/dispatch failure stops the Mailman and drains queued output; it
has no implicit retry. Overflow rejects only that emit. A successful emit means
outbox admission, not delivery or successful callback completion.

## Delivery policies

A topic has one bounded delivery policy.

### RELIABLE

Every active subscriber is admitted before the publication starts. Per-subscriber
outstanding delivery is bounded. Once the bound is exhausted, publication fails
with EventBackpressureException instead of allocating an unbounded pending-writer
queue.

A successful publication returns a PublishReceipt whose completion Future
resolves when every reliable channel write has been admitted. Waiting for this
Future never blocks an OresVM carrier thread.

Reliable delivery is therefore backpressure, not infinite buffering. Outstanding
buffered plus waiting deliveries are bounded per subscriber by max(capacity + 1,
2 * capacity). Cancelling a publication receipt cancels its pending channel writes;
already admitted deliveries remain readable. Receipt completion acknowledges
admission, not processing, and does not provide persistence or replay.

readAsync also bounds pending reads by the subscribing actor's maxMailboxMessages
policy. Exceeding that limit rejects before installing another channel waiter.
Completion, cancellation, or subscription close releases the pending-read slot.
Cancelling an unread wait does not consume the next event.

### LATEST

LATEST topics have capacity 1. Publishing a newer unread event coalesces the
older event. This is appropriate for monotonic/incumbent-style knowledge such as
current best bounds, latest policy versions, or current consensus state.

### LOSSY

LOSSY topics never wait. A full subscriber queue drops the new event and records
the drop in publication/topic counters. This is intended for telemetry or other
information where freshness matters more than completeness.

## Runtime-owned reductions

The first collective operation is a monotonic double reduction:

- MIN
- MAX

A reduction owns its state in the runtime rather than in any actor. An improving
candidate updates the reduction and publishes a LATEST notification; a
non-improving candidate is ignored. State mutation and notification are ordered
under one reduction-local critical section so concurrent improvements cannot
publish an older bound after a newer one.

The reduction lock is local to that reduction. It is not a global event-bus lock.
Reduction notification topics are runtime-owned: publish and publishSystem reject
these names; only reduceDouble may produce notifications. An ordinary topic cannot
be converted into a reduction. State commits after the topic's close/admission fence
and before notification delivery, so a rejected admission cannot silently advance
its bound.

## Payload ownership and memory

Event values are data-only. Publishing:

1. rejects null and live capabilities such as ActorRef, Shared, SyncCell, mutex
   handles, and ActorGroup join capabilities;
2. freezes the payload once before fanout;
3. reserves the retained payload size against ActorRuntime's aggregate actor
   memory ceiling;
4. fans out the immutable value to subscriber channels;
5. releases retained payload accounting as queued deliveries are read,
   coalesced, cancelled, rejected, or torn down.

This keeps the hot path zero-copy for the current supported subscriber domain
without granting actors shared mutable state.

The accounting is intentionally conservative and represents OresVM policy, not
a claim about exact JVM object layout.

## Isolation boundary and per-subscriber copied delivery (PR #519)

SHARED actors continue to subscribe to the one-immutable-snapshot fanout. Their
events share the same immutable graph and its aggregate-runtime reservation.

PRIVATE and UNTRUSTED subscribers may now subscribe: publication constructs
an **independent deeply copied event graph for each isolated subscriber**
rather than forwarding the SHARED payload graph, and charges each copy to
that destination actor's private-memory slice and the runtime aggregate cap.
The copy's lease is released exactly once on read, coalesce, cancellation,
subscription close, actor termination, or group close. RELIABLE publications
prepare every isolated copy before starting subscriber delivery so failed
preflight allocations do not create an avoidable partial broadcast.

Only data-only values are eligible. Actor references, group join capabilities,
shared memory, mutexes, native resources, FDs and arbitrary host objects
are forbidden as events. UNTRUSTED inbound validation applies to each copied
payload.

**Security limitation:** The existing JVM ActorMemorySlice + independent Java
object graph provides logical isolation and accounting, **not** a separate
OS process, hardware address-space boundary or validated hostile-code
sandbox. A future physically isolated worker backend must serialize the event
over a bounded transport, and retain exactly these per-recipient lifetime and
quota guarantees. Do not expose the same host Event or Channel object to
untrusted guest code.

## ActorGroup scope and membership boundaries (PR #519)

`ActorGroupScope.TREE` is anchored to the highest actor ancestor in the
creating actor's *actual* supervisor ancestry. Its join capability alone is
not sufficient to admit an actor from another root tree. The creator's death
still closes the group.

`ActorGroupScope.VM` is a host/supervisor-created, one-runtime-local group,
suitable for explicitly authorized cross-tree cooperation.

`ActorGroupScope.CLUSTER` is **fail-closed** and throws UnsupportedOperationException.
No cross-node membership, delivery, ordering, persistence or partition
guarantees exist yet. Introducing CLUSTER requires authenticated node identities,
fencing generations, bounded transport, membership reconciliation and explicit
at-most-once/at-least-once/exactly-once boundaries.

The zero-argument runtime `createActorGroup()` retains backward compatibility:
inside an actor it defaults to TREE, and for host/supervisor creation it
defaults to VM. Actor-group scopes are runtime Java APIs in this draft; there
is not yet source-language `ActorGroup.new(scope: ...)` lowering.


## Performance contract

No source-level latency number such as "10 ms" is guaranteed. OresVM should make
same-process communication much faster than that, but latency depends on carrier
placement, CPU topology, queue contention, payload size, and subscriber count.

The current ChannelRuntime implementation is the correctness substrate. It is
not the final NUMA-aware physical design. Future runtime work may replace the
backend with:

- per-worker or per-NUMA event shards;
- cache-line-aligned SPSC/MPMC queues;
- native ring buffers;
- topic IDs instead of string lookup on the hot path;
- batched fanout;
- topology-aware subscriber placement;
- isolated-copy delivery for private/untrusted actors;
- CPU/GPU buffer handles for large data instead of copying tensor contents.

Those changes must preserve the semantics in this document.

## Intended algorithm families

ActorGroup events are specifically useful for fine-grained cooperative
algorithms such as parallel branch-and-bound/branch-and-cut, SAT/CP clause
sharing, parallel best-first search, MCTS, asynchronous graph propagation,
ADMM/coordinate methods, large-neighborhood search, particle/swarm methods, and
other solvers where independently scheduled workers benefit from prompt shared
knowledge.

The group should carry small control/knowledge messages (bounds, cuts, IDs,
versions, handles, statistics), not repeatedly copy large matrices or tensors.
Large numerical state should use OresVM-owned immutable/buffer handles and
data-oriented CPU/GPU kernels.
