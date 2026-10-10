package dev.oreslang;

import dev.oreslang.compiler.OresCompiler;
import dev.oreslang.parser.Parser;
import org.junit.jupiter.api.Test;

import java.io.InputStream;
import java.nio.charset.StandardCharsets;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Real Oreslang parser/type-check admission of the exact shared REST contract.
 * Does not claim GraphQL/RPC/Lambda execution, deployed HTTP, or JVM-native AOT.
 */
final class SharedRestRouteContractAdmissionTest {
    private static final String ROUTE_IMPL = """
            define module Route conforms RouteContract as
              pub fnc get(String request): String { return request; }
              pub fnc post(String request): String { return request; }
            end
            """;

    private static String sharedContract() throws Exception {
        try (InputStream stream = SharedRestRouteContractAdmissionTest.class
                .getResourceAsStream("/oreslang/shared/RouteContract.ores")) {
            assertNotNull(stream, "pinned shared RouteContract resource is missing");
            return new String(stream.readAllBytes(), StandardCharsets.UTF_8);
        }
    }

    @Test
    void exactPinnedSharedContractAndRealConformingModuleTypeCheck() throws Exception {
        String contract = sharedContract();
        assertDoesNotThrow(() -> Parser.parse(contract));
        assertDoesNotThrow(() -> OresCompiler.parseAndTypeCheck(contract + "\n" + ROUTE_IMPL));
    }

    @Test
    void missingPostMethodCannotSatisfySharedContract() throws Exception {
        String invalid = """
                define module Route conforms RouteContract as
                  pub fnc get(String request): String { return request; }
                end
                """;
        String source = sharedContract() + "\n" + invalid;
        assertThrows(IllegalArgumentException.class,
                () -> OresCompiler.parseAndTypeCheck(source));
    }

    @Test
    void contractArrowAndModuleConformsToAreRejected() throws Exception {
        String badArrow = sharedContract().replace(
                "fnc get(String request) => String;",
                "fnc get(String request) -> String;");
        assertThrows(IllegalArgumentException.class, () -> Parser.parse(badArrow));

        String badConforms = ROUTE_IMPL.replace(
                "Route conforms RouteContract", "Route conforms to RouteContract");
        assertThrows(IllegalArgumentException.class,
                () -> Parser.parse(sharedContract() + "\n" + badConforms));
    }

    @Test
    void incorrectReturnTypeCannotPassImplementationConformance() throws Exception {
        String badType = ROUTE_IMPL.replace(
                "pub fnc post(String request): String",
                "pub fnc post(String request): int");
        assertThrows(IllegalArgumentException.class,
                () -> OresCompiler.parseAndTypeCheck(sharedContract() + "\n" + badType));
    }
}
