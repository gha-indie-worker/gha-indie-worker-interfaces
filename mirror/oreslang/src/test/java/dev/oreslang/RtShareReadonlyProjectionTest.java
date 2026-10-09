package dev.oreslang;

import dev.oreslang.parser.Parser;
import dev.oreslang.types.TypeChecker;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.Source;
import org.junit.jupiter.api.Test;

import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;

import static org.junit.jupiter.api.Assertions.*;

final class RtShareReadonlyProjectionTest {
    @Test
    void mutableSourceAndNestedOwnedSnapshotNeverShareWritableLists() throws Exception {
        assertEquals("12", run("""
                pub routine main(): void {
                  val Array<int> source = new Array<int>();
                  source.add(7);
                  val snapshot = rt share source;
                  source.add(9);
                  stdio.stdout.write(snapshot.size);
                  stdio.stdout.write(source.size);
                  return;
                }
                """));
    }

    @Test
    void readonlyProjectionDoesNotPermitMutatorMethods() {
        IllegalArgumentException error = assertThrows(IllegalArgumentException.class,
                () -> TypeChecker.check(Parser.parse("""
                    pub routine main(): void {
                      val Array<int> source = new Array<int>();
                      val snapshot = rt share source;
                      snapshot.add(5);
                      return;
                    }
                    """)));
        assertTrue(error.getMessage().contains("readonly"), error.getMessage());
    }

    @Test
    void readonlyCapabilityCannotBeForgedByCast() {
        IllegalArgumentException error = assertThrows(IllegalArgumentException.class,
                () -> TypeChecker.check(Parser.parse("""
                    pub routine main(): void {
                      val Array<int> source = new Array<int>();
                      val pretend = source as Readonly<Array<int>>;
                      return;
                    }
                    """)));
        assertTrue(error.getMessage().contains("Readonly"), error.getMessage());
    }

    @Test
    void readonlyClassSnapshotUsesDetachedFieldStorage() throws Exception {
        assertEquals("79", run("""
                define class Box as
                  pub let int value = 7;
                end
                pub routine main(): void {
                  val Box source = new Box();
                  val snapshot = rt share source;
                  source.value = 9;
                  stdio.stdout.write(snapshot.value);
                  stdio.stdout.write(source.value);
                  return;
                }
                """));
    }

    private static String run(String code) throws Exception {
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        try (Context context = Context.newBuilder("ores")
                .allowAllAccess(true).out(output).build()) {
            context.eval(Source.newBuilder("ores", code, "readonly-projection.ores").build());
        }
        return output.toString(StandardCharsets.UTF_8);
    }
}
