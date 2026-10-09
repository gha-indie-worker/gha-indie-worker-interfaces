package dev.oreslang.runtime;

import dev.oreslang.ast.Ast;

import java.util.Map;
import java.util.Objects;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;

/**
 * Process-local, OresVM-owned read-only source code images.
 *
 * <p>One immutable AST/linked code image is installed per code-unit identity,
 * irrespective of the number or isolation kind of actors evaluating it. Actor
 * state, environment frames, mailbox payloads and capability policy are NOT
 * installed here: they remain independently confined to their owner.</p>
 *
 * <p>This store shares JVM/Truffle AST code data inside one OresVM/OS process.
 * It does not claim sharing compiled native instructions across processes,
 * Truffle Engines or independent hot-reload generations.</p>
 */
public final class SharedCodeImageStore implements AutoCloseable {

    /**
     * Opaque capability minted only by this store. Callers may observe the
     * immutable program identity but cannot manufacture a CodeImage that
     * bypasses publication/provenance checks.
     */
    public static final class CodeImage {
        private final String codeUnitId;
        private final Ast.Program program;
        private final long generation;

        private CodeImage(String codeUnitId, Ast.Program program, long generation) {
            if (Objects.requireNonNull(codeUnitId, "codeUnitId").isBlank()) {
                throw new IllegalArgumentException("code unit id cannot be blank");
            }
            this.codeUnitId = codeUnitId;
            this.program = Objects.requireNonNull(program, "program");
            this.generation = generation;
        }

        public String codeUnitId() { return codeUnitId; }
        public Ast.Program program() { return program; }
        /** Immutable VM-local code generation identity, never reused in this store. */
        public long generation() { return generation; }

        @Override
        public String toString() {
            return "CodeImage[" + codeUnitId + "@" + generation + "]";
        }
    }

    private final Map<String, CodeImage> images = new ConcurrentHashMap<>();
    private final AtomicBoolean closed = new AtomicBoolean();
    private final AtomicLong nextGeneration = new AtomicLong();

    public CodeImage publish(String codeUnitId, Ast.Program immutableProgram) {
        synchronized (images) {
            if (closed.get()) throw new IllegalStateException("shared code store is closed");
            CodeImage existing = images.get(codeUnitId);
            if (existing != null && existing.program() == immutableProgram) return existing;
            // Never replace an executable image that existing actors may still
            // have pinned. Hot reload must link a versioned code-unit identity
            // in a new generation/context and migrate state explicitly.
            CodeImage image = new CodeImage(codeUnitId, immutableProgram,
                    nextGeneration.incrementAndGet());
            CodeImage prior = images.putIfAbsent(codeUnitId, image);
            if (prior != null && prior.program() != immutableProgram) {
                throw new IllegalStateException(
                        "attempted to replace already-linked code unit '" + codeUnitId
                        + "' without starting a new versioned code generation");
            }
            return prior == null ? image : prior;
        }
    }

    /** VM/internal lookup only. Never expose this reference to untrusted source code. */
    public CodeImage get(String codeUnitId) {
        Objects.requireNonNull(codeUnitId, "codeUnitId");
        if (closed.get()) throw new IllegalStateException("shared code store is closed");
        return images.get(codeUnitId);
    }

    public int imageCount() {
        if (closed.get()) return 0;
        return images.size();
    }

    @Override
    public void close() {
        synchronized (images) {
            if (!closed.compareAndSet(false, true)) return;
            images.clear();
        }
    }
}
