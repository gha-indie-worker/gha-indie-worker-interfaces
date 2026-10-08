#!/usr/bin/env python3
"""Verify TypeSpec-generated OpenAPI and runtime test fixtures agree, fail closed."""
import argparse
import json
from pathlib import Path
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
CONTRACTS = ROOT / "contracts" / "conformance"
REQUIRED = {
    ("GET", "/health"): ("200", "application/json", "json"),
    ("GET", "/plaintext"): ("200", "text/plain; charset=utf-8", "utf8"),
    ("GET", "/json"): ("200", "application/json", "json"),
    ("POST", "/echo"): ("200", "application/octet-stream", "echo"),
}
MAX_ECHO = 1024 * 1024


def load_cases():
    schema = json.loads((CONTRACTS / "cases.schema.json").read_text())
    Draft202012Validator.check_schema(schema)
    data = json.loads((CONTRACTS / "cases.json").read_text())
    Draft202012Validator(schema).validate(data)
    result_schema = json.loads((CONTRACTS / "result.schema.json").read_text())
    Draft202012Validator.check_schema(result_schema)
    ids = [x["id"] for x in data["scenarios"]]
    if len(set(ids)) != len(ids):
        raise ValueError("duplicate scenario ID")
    seen = set()
    for case in data["scenarios"]:
        pair = (case["method"], case["path"])
        expected = case["expected"]
        if case["inOpenApi"]:
            if pair not in REQUIRED:
                raise ValueError(f"undeclared extra contract operation: {pair}")
            status, mime, kind = REQUIRED[pair]
            if str(expected["status"]) != status:
                raise ValueError(f"mismatched required status: {case['id']}")
            if expected["headers"].get("content-type", "").lower() != mime:
                raise ValueError(f"wrong required Content-Type: {case['id']}")
            if expected["body"]["kind"] != kind:
                raise ValueError(f"wrong required body type: {case['id']}")
            seen.add(pair)
        elif pair != ("GET", "/not-found") or expected["status"] != 404:
            raise ValueError(f"unexpected unmodeled scenario: {case['id']}")
        if case["method"] == "POST":
            if "payload" not in case:
                raise ValueError(f"missing binary fixture: {case['id']}")
            if case["payload"]["byteRange"] * case["payload"]["repeat"] > MAX_ECHO:
                raise ValueError(f"echo fixture exceeds 1MiB limit: {case['id']}")
        elif "payload" in case:
            raise ValueError(f"GET request must not carry payload: {case['id']}")
    if seen != set(REQUIRED):
        raise ValueError(f"missing required fixture operations: {set(REQUIRED) - seen}")
    if len(data["scenarios"]) < 8 or "not-found" not in ids:
        raise ValueError("not-found and all 4 binary echo cases are required")
    repeats = {x.get("payload", {}).get("repeat") for x in data["scenarios"] if x["method"] == "POST"}
    if repeats != {0, 4, 256, 4096}:
        raise ValueError(f"binary echo fixtures missing expected lengths: {repeats}")
    return data


def check_openapi(doc):
    paths = doc["paths"]
    actual = {(method.upper(), path) for path, item in paths.items()
              for method in item if method.lower() in ("get", "post", "put", "patch", "delete", "head")}
    if actual != set(REQUIRED):
        raise ValueError(f"OpenAPI missing/extra routes: missing={set(REQUIRED)-actual}, extra={actual-set(REQUIRED)}")
    for (method, path), (status, mime, kind) in REQUIRED.items():
        operation = paths[path][method.lower()]
        responses = operation.get("responses", {})
        if status not in responses:
            raise ValueError(f"{method} {path} missing status {status}")
        content = responses[status].get("content", {})
        if mime not in {key.lower(): val for key, val in content.items()}:
            raise ValueError(f"{method} {path} missing media {mime}; found {list(content)}")
        if not content[mime].get("schema"):
            raise ValueError(f"{method} {path} missing response body schema")
        if method == "POST":
            types = operation.get("requestBody", {}).get("content", {})
            if "application/octet-stream" not in types:
                raise ValueError("POST /echo missing binary request body")
            if "413" not in responses:
                raise ValueError("POST /echo must declare 413 Payload Too Large")
    # Check literal JSON schemas with refs to prevent weakening required fields.
    defs = doc.get("components", {}).get("schemas", {})
    for (method, path), (status, mime, kind) in REQUIRED.items():
        if kind != "json":
            continue
        schema = paths[path][method.lower()]["responses"][status]["content"][mime]["schema"]
        if "$ref" in schema:
            name = schema["$ref"].split("/")[-1]
            schema = defs[name]
        key, literal = ("status", "ok") if path == "/health" else ("message", "hello, world!")
        prop = schema.get("properties", {}).get(key, {})
        if key not in schema.get("required", []):
            raise ValueError(f"{path} response missing required JSON field {key}")
        if literal not in prop.get("enum", [prop.get("const")]):
            raise ValueError(f"{path} JSON field {key} lost literal constraint")
    return True


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--openapi", type=Path, default=None)
    args = parser.parse_args()
    cases = load_cases()
    if args.openapi:
        check_openapi(json.loads(args.openapi.read_text()))
    print(json.dumps({"status": "ok", "scenarios": len(cases["scenarios"]),
                      "required_routes": len(REQUIRED), "compiled_openapi_checked": args.openapi is not None}))


if __name__ == "__main__":
    main()
