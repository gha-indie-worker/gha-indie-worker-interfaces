package dev.oreslang.runtime;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Timeout;

import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;

import static org.junit.jupiter.api.Assertions.*;

/** Root/tree authorization and the non-consumable readiness signal contract. */
@Timeout(30)
final class ActorGroupScopeAndReadySignalTest {
    @Test
    void hostVmGroupsWorkButClusterAndHostTreeFailClosed() throws Exception {
        try (ActorRuntime runtime = new ActorRuntime()) {
            var group = runtime.createActorGroup(ActorRuntime.ActorGroupScope.VM);
            assertEquals(ActorRuntime.ActorGroupScope.VM, group.scope());
            assertTrue(group.treeRootId().isEmpty());
            assertThrows(SecurityException.class, () ->
                    runtime.createActorGroup(ActorRuntime.ActorGroupScope.TREE));
            assertThrows(UnsupportedOperationException.class, () ->
                    runtime.createActorGroup(ActorRuntime.ActorGroupScope.CLUSTER));
            var local = runtime.createActorGroup();
            assertEquals(ActorRuntime.ActorGroupScope.VM, local.scope());
            local.close();
            group.close();
        }
    }

    @Test
    void treeGroupsRejectUnrelatedActorsButAllowCreatorDescendants() throws Exception {
        try (ActorRuntime runtime = new ActorRuntime()) {
            AtomicReference<ActorRuntime.ActorGroup> created = new AtomicReference<>();
            AtomicReference<ActorRuntime.ActorGroupJoinCapability> join = new AtomicReference<>();
            AtomicReference<ActorRuntime.ActorRef<String>> descendant = new AtomicReference<>();
            AtomicReference<Throwable> failure = new AtomicReference<>();
            CountDownLatch made = new CountDownLatch(1);
            ActorRuntime.ActorRef<String> root = runtime.spawnShared(() -> (message, context) -> {
                try {
                    var group = context.runtime().createActorGroup(ActorRuntime.ActorGroupScope.TREE);
                    created.set(group);
                    join.set(group.joinCapability());
                    descendant.set(context.runtime().spawnShared(
                            ctx -> (childMessage, childContext) -> { }));
                } catch (Throwable exception) {
                    failure.set(exception);
                } finally {
                    made.countDown();
                }
            });
            root.send("start");
            assertTrue(made.await(5, TimeUnit.SECONDS));
            assertNull(failure.get(), String.valueOf(failure.get()));
            var group = created.get();
            assertNotNull(group);
            assertEquals(ActorRuntime.ActorGroupScope.TREE, group.scope());
            assertEquals(root.id(), group.treeRootId().orElseThrow());
            runtime.createActorGroup(ActorRuntime.ActorGroupScope.VM).close();

            var siblingTree = runtime.spawnShared(() -> (message, context) -> { });
            var child = descendant.get();
            child.getReadySignal().getAwaited().get(5, TimeUnit.SECONDS);
            group.join(child, join.get());
            assertEquals(2, group.memberCount());
            assertThrows(SecurityException.class, () -> group.join(siblingTree, join.get()),
                    "unrelated root actors cannot enter a TREE-scope group");
            assertEquals(2, group.memberCount());

            child.stop();
            siblingTree.stop();
            root.stop();
            assertTrue(child.awaitTermination(5, TimeUnit.SECONDS));
            assertTrue(root.awaitTermination(5, TimeUnit.SECONDS));
            assertTrue(group.closed(), "creator termination closes actor-created groups");
        }
    }

    @Test
    void readySignalsAreStableMultiWaiterAndLateWaiterSafe() throws Exception {
        try (ActorRuntime runtime = new ActorRuntime()) {
            var actor = runtime.spawnShared(() -> (String message, ActorRuntime.ActorContext<String> ctx) -> {});
            var signal = actor.getReadySignal();
            assertSame(signal, actor.getReadySignal());
            assertSame(signal.getAwaited(), actor.ready());
            signal.getAwaited().get(5, TimeUnit.SECONDS);
            actor.getReadySignal().getAwaited().get(5, TimeUnit.SECONDS);
            actor.stop();
            assertTrue(actor.awaitTermination(5, TimeUnit.SECONDS));
        }
    }

    @Test
    void readinessFailureStaysFailedForLateWaiters() throws Exception {
        try (ActorRuntime runtime = new ActorRuntime()) {
            ActorRuntime.ActorRef<String> broken = runtime.spawnShared(
                    () -> { throw new IllegalStateException("failed during actor factory"); });
            var signal = broken.getReadySignal();
            assertThrows(ExecutionException.class, () ->
                    signal.getAwaited().get(5, TimeUnit.SECONDS));
            assertSame(signal, broken.getReadySignal());
            assertThrows(ExecutionException.class, () ->
                    broken.getReadySignal().getAwaited().get(5, TimeUnit.SECONDS));
        }
    }
}
