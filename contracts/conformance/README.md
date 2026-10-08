# HTTP conformance contracts

* `main.tsp`: authoritative TypeSpec declarations for the four **successful API operations**, response statuses, media types, body schemas, and the 413 error case.
* `tspconfig.yaml`, `package.json`: compile TypeSpec to OpenAPI. Run `npm install` and `npm run compile` from this folder. CI also checks that the generated OpenAPI declares exactly the four required route/method pairs.
* `cases.json`: runtime fixtures for request bodies and expected response headers, statuses, and JSON values, plus the 404 fallback probe (not an OpenAPI route).
* `cases.schema.json`: Draft 2020-12 JSON Schema for the fixture format.
* `result.schema.json`: Draft 2020-12 JSON Schema for machine-readable conformance results.

The validator **does not infer implementation from source**. It launches the server, sends actual HTTP/1.1 requests, validates statuses, exact bodies, and normalized header media types. It checks kept-alive connections and concurrent requests. Schema acceptance is not proof of runtime compliance, and passing these fixtures is not a throughput benchmark.

The TypeSpec emitter may canonicalize Content-Type representation, and OpenAPI is used to enforce route, response, and media declarations. Runtime verification uses case-insensitive media-type comparison while preserving exact payload bytes and JSON semantics.

Install Python validation dependencies: `python3 -m pip install jsonschema==4.25.1`.

The generated OpenAPI is an output artifact, not something developers should hand-edit; CI checks it and ensures required contracts are present. For TypeSpec binary bodies, bytes refer to raw `application/octet-stream` bytes rather than a base64 JSON string.
