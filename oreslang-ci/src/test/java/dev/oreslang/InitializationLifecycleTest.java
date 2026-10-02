package dev.oreslang;

import dev.oreslang.compiler.IncrementalCompiler;
import dev.oreslang.parser.Parser;
import dev.oreslang.types.TypeChecker;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.PolyglotException;
import org.graalvm.polyglot.Source;
import org.junit.jupiter.api.Test;

import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

final class InitializationLifecycleTest {
    @Test
    void allModulesAreLinkedBeforeAnyInitRunsAndMainRunsLast() throws Exception {
        String output = run("""
                define module first
                  fnc init() => void {
                    stdio.stdout.write("A");
                    later.ping();
                  }
                end

                define module later
                  pub fnc ping() => void {
                    stdio.stdout.write("L");
                  }

                  routine init() => void {
                    stdio.stdout.write("B");
                  }
                end

                pub routine main() => void {
                  stdio.stdout.write("M");
                }
                """);

        assertEquals("ALBM", output);
    }

    @Test
    void everyInitCompletesBeforeMainStarts() throws Exception {
        String output = run("""
                define module one
                  fnc init() => void {
                    stdio.stdout.write("1");
                  }
                end

                define module two
                  routine init() => void {
                    stdio.stdout.write("2");
                  }
                end

                fnc init() => void {
                  stdio.stdout.write("R");
                }

                pub routine main() => void {
                  stdio.stdout.write("M");
                }
                """);

        assertEquals("12RM", output);
    }

    @Test
    void initFailureAbortsStartupAndMainNeverRuns() throws Exception {
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        Source source = Source.newBuilder(OresLanguage.ID, """
                fnc init() => void {
                  stdio.stdout.write("I");
                  panic "init-failed";
                }

                pub routine main() => void {
                  stdio.stdout.write("M");
                }
                """, "init-failure.ores")
                .mimeType(OresLanguage.MIME_TYPE)
                .build();

        try (Context context = Context.newBuilder(OresLanguage.ID)
                .allowAllAccess(false)
                .out(output)
                .build()) {
            assertThrows(PolyglotException.class, () -> context.eval(source));
        }

        assertEquals("I", output.toString(StandardCharsets.UTF_8));
    }

    @Test
    void initMustBeSynchronousZeroArityVoidAndNonGeneric() {
        assertInitRejected("""
                fnc init(int x) => void {
                  return;
                }
                """, "zero parameters");

        assertInitRejected("""
                fnc init<T>() => void {
                  return;
                }
                """, "generic");

        assertInitRejected("""
                fnc init() => int {
                  return 1;
                }
                """, "return void");

        assertInitRejected("""
                async fnc init() => void {
                  return;
                }
                """, "cannot be async");
    }

    @Test
    void sourceCannotInvokeInitDirectly() {
        IllegalArgumentException local = assertThrows(
                IllegalArgumentException.class,
                () -> TypeChecker.check(Parser.parse("""
                        fnc init() => void {
                          return;
                        }

                        pub routine main() => void {
                          init();
                        }
                        """)));
        assertTrue(local.getMessage().contains("cannot be called directly"));

        IllegalArgumentException qualified = assertThrows(
                IllegalArgumentException.class,
                () -> TypeChecker.check(Parser.parse("""
                        define module lifecycle
                          pub fnc init() => void {
                            return;
                          }
                        end

                        pub routine main() => void {
                          lifecycle.init();
                        }
                        """)));
        assertTrue(qualified.getMessage().contains("cannot be called directly"));
    }

    @Test
    void circularFileImportsLoadCompletelyBeforeInitPlanning() {
        IncrementalCompiler compiler = new IncrementalCompiler();
        LinkedHashMap<String, String> sources = new LinkedHashMap<>();

        sources.put("a.ores", """
                import fnc {b_helper} from "./b.ores";

                fnc init() => void {
                  return;
                }

                pub fnc a_helper() => void {
                  return;
                }

                pub routine main() => void {
                  return;
                }
                """);

        sources.put("b.ores", """
                import fnc {a_helper} from "./a.ores";

                routine init() => void {
                  return;
                }

                pub fnc b_helper() => void {
                  return;
                }
                """);

        IncrementalCompiler.BuildResult build = assertDoesNotThrow(() -> compiler.compile(sources));
        IncrementalCompiler.StartupPlan plan = build.startupPlan("a.ores");

        assertEquals(List.of("a.ores", "b.ores"), plan.loadOrder());
        assertEquals(2, plan.initializationUnitOrder().size());
        assertTrue(plan.initializationUnitOrder().containsAll(List.of("a.ores", "b.ores")));
        assertEquals(2, plan.initHooks().size());
        assertEquals("b.ores", plan.initHooks().get(0).unitId());
        assertEquals("a.ores", plan.initHooks().get(1).unitId());
    }

    @Test
    void circularStartupPlanIsDeterministicAndDependencyFirstOutsideCycles() {
        IncrementalCompiler compiler = new IncrementalCompiler();

        Map<String, String> sources = new LinkedHashMap<>();
        sources.put("entry.ores", """
                import fnc {a} from "./a.ores";
                fnc init() => void { return; }
                pub routine main() => void { return; }
                """);
        sources.put("a.ores", """
                import fnc {b} from "./b.ores";
                fnc init() => void { return; }
                pub fnc a() => void { return; }
                """);
        sources.put("b.ores", """
                import fnc {a} from "./a.ores";
                fnc init() => void { return; }
                pub fnc b() => void { return; }
                """);

        IncrementalCompiler.StartupPlan plan = compiler.compile(sources).startupPlan("entry.ores");

        assertEquals(List.of("a.ores", "b.ores", "entry.ores"), plan.loadOrder());
        assertEquals("entry.ores",
                plan.initializationUnitOrder().get(plan.initializationUnitOrder().size() - 1),
                "acyclic importer initializes after its dependency SCC");
        assertEquals(3, plan.initHooks().size());
    }

    private static void assertInitRejected(String source, String expected) {
        IllegalArgumentException error = assertThrows(
                IllegalArgumentException.class,
                () -> TypeChecker.check(Parser.parse(source)));
        assertTrue(
                error.getMessage().contains(expected),
                () -> "unexpected init validation error: " + error.getMessage());
    }

    private static String run(String program) throws Exception {
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        Source source = Source.newBuilder(OresLanguage.ID, program, "init-order.ores")
                .mimeType(OresLanguage.MIME_TYPE)
                .build();

        try (Context context = Context.newBuilder(OresLanguage.ID)
                .allowAllAccess(false)
                .out(output)
                .build()) {
            context.eval(source);
        }

        return output.toString(StandardCharsets.UTF_8);
    }
}
