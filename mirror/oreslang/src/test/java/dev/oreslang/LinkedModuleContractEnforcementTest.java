package dev.oreslang;

import dev.oreslang.compiler.IncrementalCompiler;
import org.junit.jupiter.api.Test;

import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

final class LinkedModuleContractEnforcementTest {
    private static final String MODULE = """
            define module Ledger as
              pub let int count = 0;
              pub val int version = 1;
              pub fnc add(int delta): int {
                Ledger.count = Ledger.count + delta;
                return Ledger.count;
              }
              fnc hidden(): int { return 9; }
            end
            """;

    private static void compile(String importer) {
        new IncrementalCompiler().compile(Map.of(
                "pkg/m.ores", MODULE,
                "pkg/main.ores", """
                        import module Ledger from "./m.ores";
                        %s
                        """.formatted(importer)));
    }

    @Test
    void validPublicModuleCallAndLetWriteAreLinked() {
        assertDoesNotThrow(() -> compile("""
                pub routine main(): void {
                  Ledger.count = 4;
                  stdio.stdout.write(Ledger.add(2));
                  return;
                }
                """));
    }

    @Test
    void cannotCallUnexportedModuleFunction() {
        IllegalArgumentException error = assertThrows(IllegalArgumentException.class,
                () -> compile("""
                    pub routine main(): void {
                      Ledger.hidden();
                      return;
                    }
                    """));
        assertTrue(error.getMessage().contains("non-public")
                || error.getMessage().contains("missing"), error.getMessage());
    }

    @Test
    void cannotCallPublicModuleFunctionWithWrongArity() {
        IllegalArgumentException error = assertThrows(IllegalArgumentException.class,
                () -> compile("""
                    pub routine main(): void {
                      Ledger.add();
                      return;
                    }
                    """));
        assertTrue(error.getMessage().contains("arity"), error.getMessage());
    }

    @Test
    void cannotWriteImmutableImportedModuleField() {
        IllegalArgumentException error = assertThrows(IllegalArgumentException.class,
                () -> compile("""
                    pub routine main(): void {
                      Ledger.version = 2;
                      return;
                    }
                    """));
        assertTrue(error.getMessage().contains("mutable let"), error.getMessage());
    }
}
