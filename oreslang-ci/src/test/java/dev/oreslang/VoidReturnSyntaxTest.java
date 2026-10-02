package dev.oreslang;

import dev.oreslang.ast.Ast;
import dev.oreslang.parser.Parser;
import dev.oreslang.types.TypeChecker;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

final class VoidReturnSyntaxTest {
    @Test
    void namedCallablesAcceptAllCanonicalVoidSpellings() {
        assertDoesNotThrow(() -> TypeChecker.check(Parser.parse("""
                pub fnc a() => void {
                  return;
                }

                pub void fnc b() => void {
                  return;
                }

                pub void fnc c() {
                  return;
                }

                pub void routine d() {
                  return;
                }

                pub routine e() => void {
                  return;
                }
                """)));
    }

    @Test
    void voidPrefixAndArrowMustAgree() {
        IllegalArgumentException error = assertThrows(IllegalArgumentException.class, () -> Parser.parse("""
                pub void fnc bad() => int {
                  return 1;
                }
                """));
        assertTrue(error.getMessage().contains("'void' return modifier and => return type disagree"));
    }

    @Test
    void voidPrefixCannotDecorateData() {
        assertThrows(IllegalArgumentException.class, () -> Parser.parse("""
                void val int nope = 1;
                """));
    }

    @Test
    void explicitPipeLambdaReturnTypeIsRetainedAndChecked() {
        Ast.Program program = Parser.parse("""
                fnc make() => typeof fnc() -> int {
                  return || -> int {
                    return 42;
                  };
                }
                """);

        Ast.FunctionDecl make = (Ast.FunctionDecl) program.modules().getFirst().declarations().getFirst();
        Ast.ReturnStmt returned = (Ast.ReturnStmt) make.body().getFirst();
        Ast.LambdaExpr lambda = (Ast.LambdaExpr) returned.value();

        assertTrue(lambda.hasExplicitReturnType());
        assertEquals("int", lambda.returnType().name());
        assertDoesNotThrow(() -> TypeChecker.check(program));
    }

    @Test
    void explicitParenthesizedLambdaReturnTypeWorks() {
        assertDoesNotThrow(() -> TypeChecker.check(Parser.parse("""
                fnc make() => typeof fnc(int value) -> int {
                  return (int value) -> int {
                    return value + 1;
                  };
                }
                """)));
    }

    @Test
    void omittedLambdaReturnTypeStillInfersVoid() {
        assertDoesNotThrow(() -> TypeChecker.check(Parser.parse("""
                routine run() => void {
                  val cleanup = || -> {
                    return;
                  };
                  cleanup();
                  return;
                }
                """)));
    }

    @Test
    void explicitVoidLambdaRejectsValueReturn() {
        assertThrows(IllegalArgumentException.class, () -> TypeChecker.check(Parser.parse("""
                routine run() => void {
                  val cleanup = || -> void {
                    return 1;
                  };
                  return;
                }
                """)));
    }

    @Test
    void explicitNonVoidLambdaRequiresReturnOnEveryPath() {
        assertThrows(IllegalArgumentException.class, () -> TypeChecker.check(Parser.parse("""
                routine run() => void {
                  val f = || -> int {
                    print("missing");
                  };
                  return;
                }
                """)));
    }

    @Test
    void deferAcceptsInferredAndExplicitVoidCallbacks() {
        assertDoesNotThrow(() -> TypeChecker.check(Parser.parse("""
                routine run() => void {
                  defer || -> {
                    return;
                  };
                  defer || -> void {
                    return;
                  };
                  return;
                }
                """)));
    }

    @Test
    void deferRejectsNonVoidCallbacks() {
        IllegalArgumentException error = assertThrows(IllegalArgumentException.class, () -> TypeChecker.check(Parser.parse("""
                routine run() => void {
                  defer || -> int {
                    return 1;
                  };
                  return;
                }
                """));
        assertTrue(error.getMessage().contains("defer callback must return void"));
    }
}
