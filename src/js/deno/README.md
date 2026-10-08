# Deno HTTP benchmark

Uses native `Deno.serve` without third-party packages. Run:
`deno run --allow-net --allow-env src/js/deno/server.mjs`.

Set `HOST` (default 127.0.0.1) and `PORT` (default 8080).
From repository root:
`python3 scripts/conformance.py --target js-deno -- deno run --allow-net --allow-env src/js/deno/server.mjs`.

The binary echo request is buffered to an explicit 1 MiB maximum.
