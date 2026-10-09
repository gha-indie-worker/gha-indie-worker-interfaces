package dev.oreslang.runtime;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

/**
 * Runtime trace/refinement boundary for OresScheduler turns.
 *
 * Events are emitted from actual scheduler admission, not reconstructed from
 * abstract state. A bounded trace is rejected as incomplete if it overflows.
 * Physical carrier IDs are diagnostic only, never process identities.
 */
public final class ProcessTurnTrace {
    private static final int CAPACITY = 65_536;
    public enum Kind { ENTER, AWAIT, COOPERATE, DONE, EXIT }
    public record Event(long sequence, Kind kind, long dispatchId, long carrierId) { }
    public record Snapshot(String processName, boolean singleFlight, long dropped,
                           int active, List<Event> events) { }

    private final String processName;
    private final boolean singleFlight;
    private final ArrayDeque<Event> events = new ArrayDeque<>();
    private long sequence;
    private long dropped;
    private int active;

    ProcessTurnTrace(String processName, boolean singleFlight) {
        this.processName = processName;
        this.singleFlight = singleFlight;
    }

    synchronized void enter(long dispatch, long carrier) {
        if (singleFlight && active != 0) {
            throw new IllegalStateException(
                    "single-flight guest execution violated in " + processName);
        }
        active++;
        record(Kind.ENTER, dispatch, carrier);
    }

    synchronized void transition(Kind kind, long dispatch, long carrier) {
        if (kind == Kind.ENTER || kind == Kind.EXIT || active == 0) {
            throw new IllegalStateException("invalid guest turn transition " + kind);
        }
        record(kind, dispatch, carrier);
    }

    synchronized void exit(long dispatch, long carrier) {
        if (active < 1) throw new IllegalStateException("unbalanced guest turn exit");
        record(Kind.EXIT, dispatch, carrier);
        active--;
    }

    private void record(Kind kind, long dispatch, long carrier) {
        if (events.size() == CAPACITY) {
            events.removeFirst();
            dropped++;
        }
        events.addLast(new Event(++sequence, kind, dispatch, carrier));
    }

    public synchronized Snapshot snapshot() {
        return new Snapshot(processName, singleFlight, dropped, active,
                List.copyOf(events));
    }

    /** Validate that concrete guest-turn traces refine the single-flight model. */
    public static void verify(Snapshot snapshot) {
        if (snapshot.dropped() != 0) {
            throw new IllegalStateException("incomplete turn trace; events overwritten");
        }
        Set<Long> entered = new HashSet<>();
        long previous = 0;
        int active = 0;
        for (Event event : snapshot.events()) {
            if (event.sequence() <= previous || event.dispatchId() <= 0) {
                throw new IllegalStateException("non-monotonic or invalid dispatch trace");
            }
            previous = event.sequence();
            switch (event.kind()) {
                case ENTER -> {
                    if (!entered.add(event.dispatchId())) {
                        throw new IllegalStateException("duplicate guest admission");
                    }
                    if (snapshot.singleFlight() && active != 0) {
                        throw new IllegalStateException("overlapping guest code in "
                                + snapshot.processName());
                    }
                    active++;
                }
                case EXIT -> {
                    if (!entered.remove(event.dispatchId())) {
                        throw new IllegalStateException("guest turn exited without admission");
                    }
                    active--;
                }
                case AWAIT, COOPERATE, DONE -> {
                    if (!entered.contains(event.dispatchId())) {
                        throw new IllegalStateException(
                                "guest continuation transitioned outside its admitted turn");
                    }
                }
            }
        }
        if (active != snapshot.active()) {
            throw new IllegalStateException("trace active-turn accounting mismatch");
        }
        if (active < 0 || (snapshot.singleFlight() && active > 1)) {
            throw new IllegalStateException("invalid active guest-turn count");
        }
    }
}
