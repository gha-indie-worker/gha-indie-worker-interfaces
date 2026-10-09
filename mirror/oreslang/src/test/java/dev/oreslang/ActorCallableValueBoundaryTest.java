package dev.oreslang;

import dev.oreslang.parser.Parser;
import dev.oreslang.types.TypeChecker;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

final class ActorCallableValueBoundaryTest {
    private static void valid(String source) {
        assertDoesNotThrow(() -> TypeChecker.check(Parser.parse(source)));
    }

    private static void denied(String source) {
        IllegalArgumentException failure = assertThrows(
                IllegalArgumentException.class,
                () -> TypeChecker.check(Parser.parse(source)));
        assertTrue(failure.getMessage().contains("ownership error")
                || failure.getMessage().contains("cannot pass")
                || failure.getMessage().contains("ActorRef"),
                failure.getMessage());
    }

    @Test
    void actorTypesCannotBeOrdinaryFunctionParameters() {
        denied("""
                define shared actor Worker as
                  receive(ActorMail<int> mail): void { return; }
                end
                fnc illegal(Worker worker): void { return; }
                """);
    }

    @Test
    void actorRefsCannotBeOrdinaryFunctionParameters() {
        denied("""
                define shared actor Worker as
                  receive(ActorMail<int> mail): void { return; }
                end
                fnc illegal(ActorRef<Worker> worker): void { return; }
                """);
    }

    @Test
    void nestedActorRefTypesCannotBeSmuggledThroughGenericArguments() {
        denied("""
                define shared actor Worker as
                  receive(ActorMail<int> mail): void { return; }
                end
                fnc illegal(Option<ActorRef<Worker>> worker): void { return; }
                """);
    }

    @Test
    void moduleNamespaceIsNotAFunctionArgument() {
        denied("""
                define module Math as
                  pub fnc value(): int { return 3; }
                end
                fnc consume(int value): void { return; }
                fnc illegal(): void { consume(Math); return; }
                """);
    }

    @Test
    void classNamespaceIsNotAFunctionArgument() {
        denied("""
                define class Utility as
                  pub static fnc value(): int { return 3; }
                end
                fnc consume(int value): void { return; }
                fnc illegal(): void { consume(Utility); return; }
                """);
    }

    @Test
    void bareActorNamespaceIsNotAFunctionArgument() {
        denied("""
                define shared actor Worker as
                  receive(ActorMail<int> mail): void { return; }
                end
                fnc consume(int value): void { return; }
                fnc illegal(): void { consume(Worker); return; }
                """);
    }

    @Test
    void importedModuleNamespaceIsNotAFunctionArgument() {
        denied("""
                import module Service from './dep.ores';
                fnc consume(int value): void { return; }
                fnc illegal(): void { consume(Service); return; }
                """);
    }

    @Test
    void importedClassNamespaceIsNotAFunctionArgument() {
        denied("""
                import class Utility from './dep.ores';
                fnc consume(int value): void { return; }
                fnc illegal(): void { consume(Utility); return; }
                """);
    }

    @Test
    void rtShareCannotPromoteAnActorToSharedData() {
        denied("""
                define shared actor Worker as
                  receive(ActorMail<int> mail): void { return; }
                end
                fnc illegal(): void { val value = rt share Worker; return; }
                """);
    }

    @Test
    void rtTakeCannotMoveAnActorReferenceAsData() {
        denied("""
                define shared actor Worker as
                  receive(ActorMail<int> mail): void { return; }
                end
                fnc illegal(): void {
                  val worker = spawn Worker();
                  val stolen = rt take worker;
                  return;
                }
                """);
    }

    @Test
    void moduleStaticFunctionCallsRemainAllowed() {
        valid("""
                define module Math as
                  pub fnc value(): int { return 3; }
                end
                fnc consume(int value): int { return value; }
                fnc ok(): int { return consume(Math.value()); }
                """);
    }

    @Test
    void classStaticFunctionCallsRemainAllowed() {
        valid("""
                define class Utility as
                  pub static fnc value(): int { return 3; }
                end
                fnc consume(int value): int { return value; }
                fnc ok(): int { return consume(Utility.value()); }
                """);
    }

    @Test
    void ordinaryClassInstancesAreStillDataValues() {
        valid("""
                define class Box as
                  pub let int value = 3;
                end
                fnc consume(Box value): int { return value.value; }
                fnc ok(): int { return consume(new Box()); }
                """);
    }

    @Test
    void rtCopyAndRtShareContinueToWorkForCopyScalars() {
        valid("""
                fnc ok(): int {
                  val int n = 5;
                  val int a = rt copy n;
                  val int b = rt share n;
                  return a + b;
                }
                """);
    }

    @Test
    void actorMailboxSendStillWorksWithoutPassingActorToFunction() {
        valid("""
                define shared actor Worker as
                  receive(ActorMail<int> mail): void { return; }
                end
                pub routine main(): void {
                  val worker = spawn Worker();
                  worker.send(7);
                  return;
                }
                """);
    }
}
