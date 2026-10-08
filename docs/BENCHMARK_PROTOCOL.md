# Cross-framework benchmark protocol (draft)

No performance rankings are valid until each implementation passes the same HTTP contract, runs with documented version pins, and is measured under equivalent workloads.

## HTTP contract

All test servers listen on `127.0.0.1:${PORT}` unless the runner explicitly sets a different bind address. Default port should be documented (prefer 8080). Use HTTP/1.1 with no TLS, no compression, and no external network/database dependencies in the request path.

| Request | Exact status and body | Media type |
| --- | --- | --- |
| `GET /health` | `200` / `{"status":"ok"}` | `application/json` |
| `GET /plaintext` | `200` / `hello, world!` | `text/plain; charset=utf-8` |
| `GET /json` | `200` / `{"message":"hello, world!"}` | `application/json` |
| `POST /echo` | `200` / byte-for-byte copy of request body | `application/octet-stream` |
| `GET /not-found` | `404`; record body but do not include in throughput results | implementation-defined |

Conformance must verify the exact UTF-8 / binary response bodies and status codes; accept equivalent JSON semantics if an implementation uses another valid serialization with different key order (the simple fixed fixtures currently have none). For `POST /echo`, check empty, 1 KiB, and 64 KiB payloads with NUL/non-ASCII bytes; reject silent truncation. Do not quietly substitute text encoding for binary copying.

## Measurement matrix (proposed defaults)

1. Run each endpoint as a separate workload; start with plaintext, JSON, and echo. Do not combine unrelated workloads for headline numbers.
2. Warm the server for at least 30 seconds *after readiness* and measure for at least 60 seconds; use at least five independent trials per configuration. Record the actual timings.
3. Test connection reuse **both ways**: persistent HTTP/1.1 keep-alive and one new connection per request. Compare within the same mode only.
4. Report concurrency settings (initial suggested levels: 1, 8, 32, 128) and how they map to connections and outstanding requests. Document client-side load generation, rate model, and timeouts. Check that the generator is not the bottleneck.
5. Save throughput (successful requests/s), p50/p95/p99 latency, all error/timeout counts, CPU utilization, peak/resident memory, startup time, and raw duration. Distinguish service time from end-to-end time.
6. Pin compiler/runtime/framework dependencies. Record OS/kernel, CPU model/core count, governor/frequency, RAM, runtime flags, GC configuration, server worker/thread count, source SHA, and tool versions.
7. Run the load generator on a separate host, or at minimum on isolated cores, where practical. Report topology, local loopback/network path, port reuse behavior, socket limits, and power settings.
8. For JITs, disclose interpreter/JIT tier and warmup strategy. For native builds, disclose target, optimization flags, LTO/PGO. Do not combine interpreter and optimized compiled results without labels.

## Fairness and actor semantics

* The comparison objective is the performance of equivalent HTTP APIs, *not* language marketing claims.
* Every implementation must perform the same fixed work and return the same result. No framework may use a static proxy response while another allocates or serializes from scratch unless that is an explicitly labeled optimization category for **all** implementations.
* For Spin, test shared actor dispatch and isolated actor dispatch as separate cases, with identical handler behavior; where useful also test request-per-actor vs persistent-actor dispatch.
* Do not compare the existing `oreslang-spin/scripts/benchmark.py` output directly to this matrix: that script uses new connections per request, short samples, and different handler paths for shared/isolated actors.
* `ractor` is an actor framework, not an HTTP server; disclose its chosen HTTP implementation. Phoenix is written in Elixir and runs on BEAM; `erlang/phoenix` is organizational grouping only.
* Capture changes that affect scheduling/isolation semantics in results metadata. Actor isolation and crash containment are features, not free: do not disable them in a comparison and omit that fact.
* A failing conformance test invalidates performance comparisons for that candidate until fixed.

## Result records

The future runner should write immutable machine-readable trial files to `results/` with at least `target_id`, `source_sha`, `environment`, `scenario`, `concurrency`, `connections`, `warmup_seconds`, `measurement_seconds`, `trial`, `requests_ok`, `errors`, `throughput_rps`, `latency_ms` (p50/p95/p99), `cpu`, and `rss_bytes`. No fabricated or missing samples should be converted to zeroes. Aggregates should reference raw trial data and include dispersion/confidence intervals.

The existing [Spin benchmark notes](https://github.com/ores-truffle-oreslang/oreslang-spin/blob/main/docs/BENCHMARK.md) are historical context, not cross-language evidence.

## Implementation notes (first tranche)

* The Node.js server uses the built-in `node:http` implementation; it is a runtime baseline rather than an Express/Fastify framework benchmark.
* Node.js, Gin, and Axum buffer `POST /echo` request bodies with a 1 MiB maximum. The Spin implementation presently streams bounded chunks from the HTTP capability; this is a **known workload-implementation difference**, so echo latency comparisons must be labeled accordingly or revised for semantic parity.
* Spin's draft server currently binds to 127.0.0.1:8080. The `run.sh` adapter rejects alternate ports until a verified Oreslang environmental-port API is available. The other targets accept `HOST` and `PORT`.
* Only conformance (not performance rankings) is run in automated CI. Benchmark samples require a controlled environment and results with full provenance.

## Java / Scala / Elixir targets

* `java-vertx`: Vert.x Web 4.5.34; Java 21 CI, application bytecode target 17; direct Router and BodyHandler with 1 MiB cap.
* `scala-akka`: Scala 2.13.17, Akka HTTP 10.2.10 / Akka Stream 2.6.20 (the Maven-Central-available legacy Apache 2.0 licensing line). Maven Scala compiler; bounded strict request body.
* `erlang-phoenix`: Elixir 1.18.4 and OTP 27.3 CI, Phoenix 1.8.15 on Bandit 1.12.5. Minimal Phoenix.Endpoint/Phoenix.Router/Plug.Conn; one MiB binary body.
* Different HTTP implementations may have different HTTP parsing, connection handling, runtime scheduler, and request body internals. Preserve the contracts, log the exact versions and adapter choices, and label comparisons carefully.
