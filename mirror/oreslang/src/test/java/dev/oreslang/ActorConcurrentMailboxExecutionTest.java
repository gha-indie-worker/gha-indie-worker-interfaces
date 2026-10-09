package dev.oreslang;

import dev.oreslang.parser.Parser;
import dev.oreslang.types.TypeChecker;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.Source;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Timeout;

import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;

import static org.junit.jupiter.api.Assertions.*;

@Timeout(20)
final class ActorConcurrentMailboxExecutionTest {
    @Test
    void fiveActiveMessageCapacityStillUsesOneExecutionLane() throws Exception {
        String source = """
                define actor Worker as
                  int concurrency: 5;
                  let Channel<int> release;

                  on_start(): void {
                    self.release = Channel.new<int>(1);
                    return;
                  }

                  receive(ActorMail<int> m): void {
                    if m.value == 1; do
                      m.send("first-start");
                      val int resumed = await (nb readch self.release);
                      m.send("first-finish");
                      m.done();
                      return;
                    fi
                    if m.value == 2; do
                      m.send("second");
                      writech self.release, 7;
                      m.done();
                      return;
                    fi
                    self.end();
                    return;
                  }
                end

                pub async routine main(): void {
                  val worker = spawn Worker();
                  await worker.ready;
                  worker.send(1);
                  worker.send(2);

                  val a = (await worker.outputs.next()).value.unwrap();
                  val b = (await worker.outputs.next()).value.unwrap();
                  val c = (await worker.outputs.next()).value.unwrap();
                  stdio.stdout.write(a.value);
                  stdio.stdout.write(":");
                  stdio.stdout.write(a.message_sequence.unwrap());
                  stdio.stdout.write("|");
                  stdio.stdout.write(b.value);
                  stdio.stdout.write(":");
                  stdio.stdout.write(b.message_sequence.unwrap());
                  stdio.stdout.write("|");
                  stdio.stdout.write(c.value);
                  stdio.stdout.write(":");
                  stdio.stdout.write(c.message_sequence.unwrap());

                  worker.send(3);
                  await worker.done;
                  return;
                }
                """;
        assertEquals("first-start:0|second:1|first-finish:0",
                run(source, "interleaved-actor-mail.ores"));
    }

    @Test
    void mailSendWithoutArgumentEmitsItsImmutableTransportPayload() throws Exception {
        String source = """
                define actor Worker as
                  int concurrency: 2;
                  receive(ActorMail<int> m): void {
                    m.send();
                    m.done();
                    self.end();
                    return;
                  }
                end
                pub async routine main(): void {
                  val actor = spawn Worker();
                  await actor.ready;
                  actor.send(17);
                  val output = (await actor.outputs.next()).value.unwrap();
                  stdio.stdout.write(output.value);
                  stdio.stdout.write(":");
                  stdio.stdout.write(output.message_sequence.unwrap());
                  await actor.done;
                  return;
                }
                """;
        assertEquals("17:0", run(source, "mail-self-payload.ores"));
    }

    @Test
    void invalidActorConcurrencyFailsStaticAdmission() {
        for (int value : new int[]{0, -1, 1025}) {
            String source = """
                    define actor Worker as
                      int concurrency: %d;
                      receive(ActorMail<int> m): void {
                        m.done();
                        return;
                      }
                    end
                    """.formatted(value);
            IllegalArgumentException failure = assertThrows(
                    IllegalArgumentException.class,
                    () -> TypeChecker.check(Parser.parse(source)));
            assertTrue(failure.getMessage().contains("concurrency"), failure.getMessage());
        }
    }

    @Test
    void mailDoneArityIsCheckedAtCompileTime() {
        assertThrows(IllegalArgumentException.class,
                () -> TypeChecker.check(Parser.parse("""
                    define actor Worker as
                      receive(ActorMail<int> m): void {
                        m.done(1);
                        return;
                      }
                    end
                    """)));
    }

    private static String run(String source, String name) throws Exception {
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        try (Context context = Context.newBuilder(OresLanguage.ID)
                .allowAllAccess(false)
                .out(output)
                .build()) {
            context.eval(Source.newBuilder(OresLanguage.ID, source, name)
                    .mimeType(OresLanguage.MIME_TYPE)
                    .buildLiteral());
        }
        return output.toString(StandardCharsets.UTF_8);
    }
}
