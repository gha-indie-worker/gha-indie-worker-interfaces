# Oreslang RESTful server performance benchmarks

Reproducible HTTP-server comparisons of [Oreslang Spin](https://github.com/ores-truffle-oreslang/oreslang-spin) against JavaScript, JVM, Go, Rust, and BEAM frameworks.

**Status:** implementations are being added and verified; no cross-language benchmark rankings have been generated. Spin is a draft HTTP/1.1 framework and its existing local benchmark is not a cross-framework performance result.

## Implementations

| Ecosystem | Frameworks / runtimes |
| --- | --- |
| JavaScript | `src/js/nodejs`, `src/js/bun`, `src/js/deno` |
| Java | `src/java/vertx`, `src/java/springboot` |
| Go | `src/golang/gin`, `src/golang/mux` |
| Scala | `src/scala/akka` |
| Erlang / BEAM | `src/erlang/otp`, `src/erlang/phoenix` |
| Rust | `src/rust/axum`, `src/rust/actix`, `src/rust/ractor` |
| Oreslang baseline | `src/oreslang/spin` (reference adapter; upstream implementation lives in oreslang-spin) |

Phoenix is an **Elixir** web framework running on the Erlang VM (BEAM). It is grouped under `erlang/` as requested, not presented as an Erlang-native framework. Ractor is an actor framework; its eventual HTTP integration must be named and disclosed to avoid confusing a framework-versus-server comparison.

All fourteen targets now have runnable source or reference adapters. An `implemented` label does **not** mean validated: each adapter must pass the contract suite, then receive reproducible lockfiles and performance measurements before a ranking can be published. Some historical `.gitkeep` files remain harmless in populated directories.

## Common contract

All candidates must implement the *same behavior* before results are published:

| Endpoint | Expected response |
| --- | --- |
| `GET /health` | HTTP 200, UTF-8 JSON body `{"status":"ok"}` |
| `GET /plaintext` | HTTP 200, UTF-8 text body `hello, world!` |
| `GET /json` | HTTP 200, UTF-8 JSON body `{"message":"hello, world!"}` |
| `POST /echo` | HTTP 200, byte-for-byte copy of the supplied request body; binary payloads supported |

The implementation must bind to a configurable local port and expose clearly documented startup and shutdown commands. Do not add caches, middleware, work, or headers to one implementation that are absent from the equivalent comparison unless a separate experiment specifically measures their impact. Include a fixed-size request-body case and an empty-body case.

See [docs/BENCHMARK_PROTOCOL.md](docs/BENCHMARK_PROTOCOL.md) for validity criteria, environment capture, and reporting requirements.

## Benchmark policy

* First test correctness; record failures and error rates, not just successful requests.
* Keep warm-up, measurements, concurrency, connection reuse, payload, and HTTP protocol identical for every server.
* Separate **HTTP/server** overhead from **per-request actor creation** overhead. Spin shared/isolated actor modes are separate rows; do not compare different handler behavior as though identical.
* Pin and report language/runtime/compiler/framework versions and Git commit SHAs. Do not silently upgrade dependencies within a run.
* Report requests/second, p50/p95/p99 latency, error rate, CPU, RSS, and measurement methodology. Prefer multiple independent trials and a distinct load-generator process/host.
* Avoid publishing winner claims until identical workloads have passed the conformance suite and been reproduced.

## Oreslang Spin

Spin source: https://github.com/ores-truffle-oreslang/oreslang-spin

The upstream repo currently provides `scripts/benchmark.py`, `tests/server.ores`, and `docs/BENCHMARK.md`. Its initial localhost test creates a **new HTTP/1.1 connection per request** and measures shared and isolated actor paths that have different handler implementations. That result is *not* directly comparable to a keep-alive benchmark in another language. The adapter under `src/oreslang/spin` will eventually run the **same contract** as every other candidate while recording the exact upstream source/compiler pins.

## Next work

1. Run the full conformance matrix in a funded CI environment and fix any implementation failures.
2. Commit generated dependency lockfiles after successful resolution and record all runtime/toolchain versions.
3. Add a runner that captures hardware/software provenance and repeatable, machine-readable results.
4. Run separate persistent-connection and new-connection HTTP/1.1 suites, then publish validated results with complete raw data.

## Initial servers (not yet verified)

The Node.js, Gin, and Axum implementations and the two Spin actor-mode adapters expose the common four endpoints. Conformance tests must pass before these become verified targets. The CI tests correctness, not performance.

```bash
python3 scripts/conformance.py -- node src/js/nodejs/server.mjs
(cd src/golang/gin && go mod tidy && go build -o server .)
python3 scripts/conformance.py -- ./src/golang/gin/server
(cd src/rust/axum && cargo build --release)
python3 scripts/conformance.py -- ./src/rust/axum/target/release/oreslang-benchmark-axum
```

Spin requires local private pinned compiler/framework checkouts; see [its adapter README](src/oreslang/spin/README.md).

**Known gaps:** Pin transitive dependency locks (`go.sum`, `Cargo.lock`) after validated resolution; compile and run Spin against its exact upstream tree; make Spin's echo body buffering equivalent to the other three before measuring echo latency. Do not publish rankings from incomplete or unverified comparisons.

## JVM and BEAM benchmark targets

The following source targets are implemented and have their own conformance jobs:

| Framework | Implementation | Build | Run |
| --- | --- | --- | --- |
| Vert.x Web 4.5.34 | `src/java/vertx` | `mvn -B -q -DskipTests package dependency:build-classpath -Dmdep.outputFile=target/classpath.txt` | `bash src/java/vertx/run.sh` |
| Akka HTTP 10.2.10 | `src/scala/akka` | same Maven command from its own directory | `bash src/scala/akka/run.sh` |
| Phoenix 1.8.15 / Bandit 1.12.5 | `src/erlang/phoenix` | `mix deps.get && mix compile` from its own directory | `bash src/erlang/phoenix/run.sh` |

Run the shared `python3 scripts/conformance.py -- bash .../run.sh` command after building. The Akka target deliberately uses the older Apache-2 licensed libraries that resolve from Maven Central; do not label it the current licensed Akka stack. Phoenix is an Elixir implementation on BEAM, not an Erlang-only program. Commit generated dependency locks after successful resolution to make the next results reproducible.

## Schema-driven conformance

`contracts/conformance/main.tsp` is the TypeSpec HTTP API with four explicit required routes and response status/Content-Type/body declarations. The CI `contracts` job emits OpenAPI and checks that none of the four methods or the expected response contracts were omitted.

`contracts/conformance/cases.json` describes eight cases, including the fallback 404, four byte-exact echo payload sizes, the 1 MiB boundary, JSON literals, required headers, and status codes. Draft 2020-12 schemas validate this fixture file and the machine-readable test results. Each runnable server executes these fixtures plus keep-alive and 24 concurrent requests (35 total checks).

```bash
(cd contracts/conformance && npm install --no-audit --no-fund && npm run compile)
python3 -m pip install jsonschema==4.25.1
python3 scripts/check_contracts.py --openapi contracts/conformance/generated/openapi.json
python3 -m unittest discover -s scripts -p 'test_contracts.py' -v
python3 scripts/conformance.py --target js-nodejs -- node src/js/nodejs/server.mjs
```

Until transitive lockfiles are committed, direct dependencies are pinned but npm/Maven/Go/Mix/Cargo dependency resolution has known reproducibility gaps. Benchmarks must not be published based on conformance pass rates alone.

## Remaining framework implementations (added, unverified)

All seven prior placeholders now contain real servers and have separate conformance CI jobs:

| Target | Dependencies / implementation | Build / run |
| --- | --- | --- |
| Bun | Native `Bun.serve` (no framework) | `bun src/js/bun/server.mjs` |
| Deno | Native `Deno.serve` (no framework) | `deno run --allow-env --allow-net src/js/deno/server.mjs` |
| Spring Boot | 3.5.6 / embedded Tomcat | `(cd src/java/springboot && mvn -B -q -DskipTests package)`; `bash src/java/springboot/run.sh` |
| Gorilla/mux | Go `net/http` + mux 1.8.1 | `(cd src/golang/mux && go mod tidy && go build -o server .)`; `src/golang/mux/server` |
| Erlang/OTP | Cowboy 2.13.0 + Ranch HTTP transport | `(cd src/erlang/otp && rebar3 compile)`; `bash src/erlang/otp/run.sh` |
| Actix Web | Actix Web 4.15.0 | `(cd src/rust/actix && cargo build --release)`; `src/rust/actix/target/release/oreslang-benchmark-actix` |
| Ractor | Ractor 0.16.5 actor + Axum 0.8.9 HTTP transport | `(cd src/rust/ractor && cargo build --release)`; `src/rust/ractor/target/release/oreslang-benchmark-ractor` |

Use `python3 scripts/conformance.py --target TARGET-ID -- COMMAND` from the repository root, or see the specific directory README. These are **not verified until real runnable tests pass**.

Ractor + Axum deliberately routes requests through one persistent actor's mailbox; it measures **actor processing plus Axum transport**, not a standalone Ractor HTTP framework. Erlang/OTP similarly uses Cowboy as its HTTP transport. Neither result should be conflated with a pure HTTP framework result or used to infer actor-per-request overhead.
