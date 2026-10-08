# Actix Web benchmark

Pinned Actix Web server with no logging, middleware, or business logic.
Build: `cd src/rust/actix && cargo build --release`.
Run from repo root: `src/rust/actix/target/release/oreslang-benchmark-actix`.
Verify: `python3 scripts/conformance.py --target rust-actix -- ./src/rust/actix/target/release/oreslang-benchmark-actix`.

`web::PayloadConfig` caps extraction at 1 MiB. The payload is fully buffered for echo, matching the other HTTP implementations.
