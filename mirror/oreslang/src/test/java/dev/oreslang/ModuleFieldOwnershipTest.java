package dev.oreslang;

import dev.oreslang.parser.Parser;
import dev.oreslang.types.TypeChecker;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.Source;
import org.junit.jupiter.api.Test;

import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;

import static org.junit.jupiter.api.Assertions.*;

final class ModuleFieldOwnershipTest {
    @Test
    void letModuleFieldPersistsBetweenCallsWithoutReevaluatingInitializer()
            throws Exception {
        assertEquals("12", run("""
                define module Counter as
                  pub let int count = 0;
                  pub fnc next(): int {
                    Counter.count = Counter.count + 1;
                    return Counter.count;
                  }
                end
                pub routine main(): void {
                  stdio.stdout.write(Counter.next());
                  stdio.stdout.write(Counter.next());
                  return;
                }
                """));
    }

    @Test
    void immutableModuleFieldsCannotBeReassigned() {
        IllegalArgumentException failure = assertThrows(
                IllegalArgumentException.class,
                () -> TypeChecker.check(Parser.parse("""
                    define module Settings as
                      pub val int version = 1;
                    end
                    pub routine main(): void {
                      Settings.version = 2;
                      return;
                    }
                    """)));
        assertTrue(failure.getMessage().contains("module field"), failure.getMessage());
    }

    private static String run(String source) throws Exception {
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        try (Context context = Context.newBuilder("ores")
                .allowAllAccess(true)
                .out(output)
                .build()) {
            context.eval(Source.newBuilder("ores", source, "module-field-ownership.ores").build());
        }
        return output.toString(StandardCharsets.UTF_8);
    }
}
