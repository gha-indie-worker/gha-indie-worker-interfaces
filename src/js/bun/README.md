# Bun HTTP benchmark

Uses the built-in Bun.serve HTTP/1.1 implementation, no NPM dependencies.
Run: `bun src/js/bun/server.mjs`.
Set `HOST` (default 127.0.0.1) and `PORT` (default 8080).
From the repository root: `python3 scripts/conformance.py --target js-bun -- bun src/js/bun/server.mjs`.

The 1 MiB echo request body is explicitly buffered and returned byte-for-byte; over-limit requests return 413. This is a native runtime benchmark, not a Bun web-framework benchmark.
