# gha-indie-worker-interfaces

Data-only contracts for Independent GitHub Actions worker and clone-server control plane. Schema: `schema/v1`. Generated TypeScript and Dart live under `generated/` and must stay types-only.

## HTTP route keys (oresoftware/api-docs)

Operation keys live in `route-maps/api.route-map.json`. That JSON is the shared
source; JSON Schema (`scripts/vendor/route-map.schema.json`) is the contract.
`python3 scripts/generate-routes.py --map route-maps/api.route-map.json --out generated/routes`
emits compile-time objects in Rust (`RouteKey`), TypeScript (`Routes`), Dart
(`Routes.byKey`), and Gleam (`RouteKey`). Frontend code uses keys instead of
path strings; a missing backend `match` / `case` / `RouteHandlers` arm fails
to compile.

Maps travel between devices as opto-sync envelopes (`ores.api-docs.route-map`).
The route map uses opto-sync; opto-sync does not depend on this RPC layer.
See https://github.com/oresoftware/api-docs

