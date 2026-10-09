package dev.oreslang.runtime;

import dev.oreslang.parser.Parser;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Timeout;

import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;

import static org.junit.jupiter.api.Assertions.*;

/**
 * A message-concurrency setting is not permission for parallel native guest
 * execution. Covers SHARED, PRIVATE and UNTRUSTED source-executor turns.
 */
@Timeout(30)
final class ActorNativeCarrierLeaseTest {

    @Test
    void foreignSupervisorStopCannotReclaimActorOwnedStateDuringActiveNativeTurn()
            throws Exception {
        for (ActorRuntime.ActorKind kind : ActorRuntime.ActorKind.values()) {
            CountDownLatch entered = new CountDownLatch(1);
            CountDownLatch release = new CountDownLatch(1);
            AtomicReference<Throwable> failure = new AtomicReference<>();
            try (SharedCodeImageStore store = new SharedCodeImageStore();
                 ActorRuntime runtime = new ActorRuntime(
                         IsolatePolicy.developer(),
                         new ActorRuntime.DispatcherConfig(2, 2, 2, 128))) {
                var image = store.publish("active-stop-" + kind + ".ores",
                        Parser.parse("pub routine main(): void { return; }"));
                ActorRuntime.ActorCodeExecutor bridge = new ActorRuntime.ActorCodeExecutor() {
                    @Override public SharedCodeImageStore.CodeImage codeImage() {
                        return image;
                    }
                    @Override public ActorRuntime.ActorOwnedGuestState initializeActor(
                            String type, ActorRuntime.ActorContext<Object> context) {
                        assertTrue(ActorRuntime.currentThreadOwnsActorNativeLease());
                        return context.self()::id;
                    }
                    @Override public void receiveActor(
                            String type, ActorRuntime.ActorOwnedGuestState state,
                            ActorRuntime.ActorInboxMail<Object> mail,
                            ActorRuntime.ActorContext<Object> context) {
                        try {
                            assertTrue(ActorRuntime.currentThreadOwnsActorNativeLease());
                            Object initial = runtime.currentActorModuleState(
                                    "pkg/private.ores", "Counter", image.generation(),
                                    java.util.ArrayList::new);
                            entered.countDown();
                            assertTrue(release.await(5, TimeUnit.SECONDS));
                            assertTrue(ActorRuntime.currentThreadOwnsActorNativeLease(),
                                    "supervisor stop cannot steal the native actor lease");
                            assertSame(mail, context.currentMail().orElseThrow(),
                                    "foreign stop must not clear active mail during actor code");
                            assertSame(initial, runtime.currentActorModuleState(
                                    "pkg/private.ores", "Counter", image.generation(),
                                    java.util.ArrayList::new),
                                    "actor-owned mutable memory cannot be retired while running");
                        } catch (Throwable error) {
                            failure.set(error);
                        }
                    }
                };
                var actor = runtime.spawnCodeActor(kind, bridge, "Worker");
                try {
                    actor.ready().get(5, TimeUnit.SECONDS);
                    actor.send("run");
                    assertTrue(entered.await(5, TimeUnit.SECONDS));
                    assertFalse(ActorRuntime.currentThreadOwnsActorNativeLease());
                    actor.stop();  // Foreign CONTROL/supervisor shutdown.
                    release.countDown();
                    assertTrue(actor.awaitTermination(5, TimeUnit.SECONDS));
                    assertNull(failure.get(), String.valueOf(failure.get()));
                } finally {
                    release.countDown();
                }
            }
        }
    }

    @Test
    void fiveSuspendedMessagesShareOneExclusiveNativeLeaseForAllActorKinds()
            throws Exception {
        for (ActorRuntime.ActorKind kind : ActorRuntime.ActorKind.values()) {
            AtomicInteger executing = new AtomicInteger();
            AtomicInteger maxExecuting = new AtomicInteger();
            CountDownLatch admitted = new CountDownLatch(5);
            CountDownLatch sixth = new CountDownLatch(1);
            CountDownLatch resumed = new CountDownLatch(5);
            List<OresFuture<Void>> suspended = new CopyOnWriteArrayList<>();

            Runnable inActor = () -> {
                assertTrue(ActorRuntime.currentThreadOwnsActorNativeLease(),
                        kind + " must own its native carrier before any actor code");
                int currentlyRunning = executing.incrementAndGet();
                maxExecuting.accumulateAndGet(currentlyRunning, Math::max);
                assertEquals(1, currentlyRunning,
                        "no two native threads may run one " + kind + " actor");
                executing.decrementAndGet();
            };

            try (SharedCodeImageStore store = new SharedCodeImageStore();
                 ActorRuntime runtime = new ActorRuntime(
                         IsolatePolicy.developer(),
                         new ActorRuntime.DispatcherConfig(4, 4, 4, 128))) {
                var image = store.publish("native-lease-" + kind + ".ores",
                        Parser.parse("pub routine main(): void { return; }"));
                ActorRuntime.ActorCodeExecutor bridge = new ActorRuntime.ActorCodeExecutor() {
                    @Override public SharedCodeImageStore.CodeImage codeImage() {
                        return image;
                    }
                    @Override public ActorRuntime.ActorOwnedGuestState initializeActor(
                            String actorType, ActorRuntime.ActorContext<Object> context) {
                        inActor.run();
                        runtime.configureCurrentActorConcurrency(5);
                        return context.self()::id;
                    }
                    @Override public void receiveActor(
                            String actorType, ActorRuntime.ActorOwnedGuestState state,
                            ActorRuntime.ActorInboxMail<Object> mail,
                            ActorRuntime.ActorContext<Object> context) {
                        inActor.run();
                        if (mail.value().equals("hold")) {
                            OresFuture<Void> pending = new OresFuture<>();
                            var target = runtime.captureCurrentContinuationTarget();
                            runtime.enqueueOnCompletion(pending, target, (ignored, failure) -> {
                                inActor.run();
                                assertNull(failure);
                                resumed.countDown();
                            });
                            runtime.suspendCurrentUserMailboxUntil(pending);
                            suspended.add(pending);
                            admitted.countDown();
                        } else if (mail.value().equals("sixth")) {
                            sixth.countDown();
                        } else {
                            throw new IllegalArgumentException("unexpected mail");
                        }
                    }
                };
                var ref = runtime.spawnCodeActor(kind, bridge, "Worker");
                try {
                    ref.ready().get(5, TimeUnit.SECONDS);
                    for (int i = 0; i < 5; i++) ref.send("hold");
                    assertTrue(admitted.await(5, TimeUnit.SECONDS),
                            kind + " should admit five unfinished messages");
                    assertEquals(5, suspended.size());
                    ref.send("sixth");
                    assertFalse(sixth.await(120, TimeUnit.MILLISECONDS),
                            kind + " exceeded its message-concurrency bound");
                    // External producer settles a future, but never executes
                    // actor code: the callback is queued on the actor lease.
                    assertFalse(ActorRuntime.currentThreadOwnsActorNativeLease());
                    suspended.getFirst().completeFromRuntime(null);
                    assertTrue(sixth.await(5, TimeUnit.SECONDS),
                            kind + " failed to admit after message completion");
                    for (int i = 1; i < 5; i++) {
                        suspended.get(i).completeFromRuntime(null);
                    }
                    assertTrue(resumed.await(5, TimeUnit.SECONDS),
                            kind + " failed to run suspended callbacks under lease");
                    assertEquals(1, maxExecuting.get());
                    assertEquals(0, executing.get());
                } finally {
                    ref.stop();
                    assertTrue(ref.awaitTermination(5, TimeUnit.SECONDS));
                }
            }
        }
    }
}
