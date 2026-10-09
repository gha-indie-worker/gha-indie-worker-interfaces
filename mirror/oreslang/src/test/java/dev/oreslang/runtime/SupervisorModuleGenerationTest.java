package dev.oreslang.runtime;

import dev.oreslang.ast.Ast;
import dev.oreslang.parser.Parser;
import org.junit.jupiter.api.Test;

import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.atomic.AtomicInteger;

import static org.junit.jupiter.api.Assertions.*;

final class SupervisorModuleGenerationTest {
    @Test
    void siblingSupervisorsHaveDistinctPersistentFieldsAndRetireOnClose() {
        try (OresVM vm = OresVM.create(Runnable::run);
             OresScheduler first = vm.createSupervisorScheduler("first");
             OresScheduler second = vm.createSupervisorScheduler("second")) {
            Object left = first.startSync(() -> vm.currentSupervisorModuleState(
                    "pkg/m.ores", "M", 1L, ArrayList<String>::new)).join();
            Object leftAgain = first.startSync(() -> vm.currentSupervisorModuleState(
                    "pkg/m.ores", "M", 1L, ArrayList<String>::new)).join();
            Object right = second.startSync(() -> vm.currentSupervisorModuleState(
                    "pkg/m.ores", "M", 1L, ArrayList<String>::new)).join();

            assertSame(left, leftAgain);
            assertNotSame(left, right);
            @SuppressWarnings("unchecked")
            List<String> leftList = (List<String>) left;
            leftList.add("first");
            assertEquals(List.of(), right);

            Object nextGeneration = first.startSync(() -> vm.currentSupervisorModuleState(
                    "pkg/m.ores", "M", 2L, ArrayList<String>::new)).join();
            assertNotSame(left, nextGeneration);

            first.close();
            assertThrows(RuntimeException.class, () -> first.startSync(() ->
                    vm.currentSupervisorModuleState("pkg/m.ores", "M", 1L, ArrayList<String>::new)).join());
        }
    }

    @Test
    void externalThreadsAndRootMayNotAccessSupervisorMemory() {
        try (OresVM vm = OresVM.create(Runnable::run)) {
            assertThrows(SecurityException.class, () ->
                    vm.currentSupervisorModuleState("pkg/m.ores", "M", 1L, ArrayList<String>::new));
            assertThrows(RuntimeException.class, () -> vm.rootScheduler().startSync(() ->
                    vm.currentSupervisorModuleState("pkg/m.ores", "M", 1L, ArrayList<String>::new)).join());
        }
    }

    @Test
    void supervisorCloseDeterministicallyReclaimsRegisteredResources() {
        AtomicInteger cleaned = new AtomicInteger();
        try (RuntimeGarbageCollector gc = new RuntimeGarbageCollector();
             OresVM vm = OresVM.create(Runnable::run)) {
            vm.setSupervisorExitHook(gc::retireProcessDomain);
            OresScheduler supervisor = vm.createSupervisorScheduler("cleanup");
            supervisor.startSync(() -> {
                // The cleanup closure intentionally does not capture the weak owner.
                Object owner = new Object();
                gc.track(owner, cleaned::incrementAndGet);
                return null;
            }).join();
            supervisor.close();
            assertEquals(1, cleaned.get(), "supervisor close must retire resource domain");
        }
    }

    @Test
    void codeImagesNeverReplaceLiveGenerationBySourceName() {
        Ast.Program first = Parser.parse("pub routine main(): void { return; }");
        Ast.Program modified = Parser.parse("pub routine main(): void { stdio.println(1); return; }");
        try (SharedCodeImageStore store = new SharedCodeImageStore()) {
            var a = store.publish("pkg/code.ores", first);
            assertSame(a, store.publish("pkg/code.ores", first));
            assertEquals(1L, a.generation());
            assertThrows(IllegalStateException.class, () ->
                    store.publish("pkg/code.ores", modified));
            var b = store.publish("pkg/code@v2.ores", modified);
            assertNotEquals(a.generation(), b.generation());
            assertSame(first, a.program());
            assertSame(modified, b.program());
        }
    }
}
