# Ractor + Axum HTTP benchmark

Pinned `ractor` 0.16.5 and `axum` 0.8.9. Axum supplies HTTP transport only; every successful route is computed by a **long-lived Ractor actor** through a mailbox and a request/reply port. This intentionally measures actor-dispatch overhead and introduces a single mailbox/serial handler bottleneck. **Do not label this benchmark "Ractor HTTP server" or compare its RPS to Axum as if they had identical workload semantics.**

Build: `cd src/rust/ractor && cargo build --release`.
Run from repo root: `src/rust/ractor/target/release/oreslang-benchmark-ractor`.
Conformance: `python3 scripts/conformance.py --target rust-ractor -- ./src/rust/ractor/target/release/oreslang-benchmark-ractor`.

Set `HOST` and `PORT`. All four routes must pass the common conformance test, including 1 MiB echo and keep-alive. Future independent experiments can add a worker-actor pool or per-request actor variant.
