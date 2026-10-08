#!/usr/bin/env python3
"""Run contract-backed HTTP conformance; this is not a performance benchmark."""
import argparse
import concurrent.futures
import http.client
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from jsonschema import Draft202012Validator

from check_contracts import CONTRACTS, load_cases

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--host", default="127.0.0.1")
parser.add_argument("--port", type=int, default=8080)
parser.add_argument("--target", default="unspecified")
parser.add_argument("--timeout", type=float, default=10.0)
parser.add_argument("command", nargs=argparse.REMAINDER)
args = parser.parse_args()
if not 1 <= args.port <= 65535 or args.timeout <= 0:
    parser.error("invalid port or timeout")
command = args.command[1:] if args.command[:1] == ["--"] else args.command
proc = None


def payload_for(case):
    spec = case.get("payload")
    if spec is None:
        return None
    return bytes(range(spec["byteRange"])) * spec["repeat"]


def request(method, path, payload=None, conn=None):
    owned = conn is None
    c = conn or http.client.HTTPConnection(args.host, args.port, timeout=args.timeout)
    try:
        headers = {"Accept-Encoding": "identity"}
        if method == "POST":
            headers["Content-Type"] = "application/octet-stream"
        c.request(method, path, body=payload, headers=headers)
        r = c.getresponse()
        raw = r.read()
        return r.status, {k.lower(): v for k, v in r.getheaders()}, raw
    finally:
        if owned:
            c.close()


def verify(case, response):
    status, headers, body = response
    expected = case["expected"]
    if status != expected["status"]:
        raise AssertionError(f"{case['id']} status: {status} != {expected['status']}")
    for key, required in expected["headers"].items():
        actual = headers.get(key, "")
        if actual.lower() != required.lower():
            raise AssertionError(f"{case['id']} header {key}: {actual!r} != {required!r}")
    spec = expected["body"]
    if spec["kind"] == "ignore":
        return
    if spec["kind"] == "utf8":
        want = spec["equals"].encode("utf-8")
        if body != want:
            raise AssertionError(f"{case['id']} UTF-8 body mismatch")
    elif spec["kind"] == "json":
        decoded = json.loads(body)
        Draft202012Validator({"type": "object", "required": list(spec["equals"]),
            "properties": {key: {"const": value} for key, value in spec["equals"].items()},
            "additionalProperties": False}).validate(decoded)
        if decoded != spec["equals"]:
            raise AssertionError(f"{case['id']} JSON value mismatch")
    elif spec["kind"] == "echo":
        if body != payload_for(case):
            raise AssertionError(f"{case['id']} binary echo mismatch {len(body)} bytes")
    else:
        raise AssertionError(f"unknown body mode: {spec['kind']}")


def wait_for_server():
    deadline = time.monotonic() + 45.0
    while time.monotonic() < deadline:
        if proc is not None and proc.poll() is not None:
            raise RuntimeError(f"server exited before readiness: {proc.returncode}")
        try:
            code, _, body = request("GET", "/health")
            if code == 200 and body == b'{"status":"ok"}':
                return
        except (OSError, http.client.HTTPException):
            pass
        time.sleep(0.15)
    raise TimeoutError(f"server not ready on {args.host}:{args.port}")


def run(cases):
    passed = 0
    failures = []
    for case in cases:
        try:
            verify(case, request(case["method"], case["path"], payload_for(case)))
            passed += 1
        except Exception as exc:
            failures.append(f"{case['id']}: {exc}")

    # The same HTTP/1.1 socket must survive 3 consecutive GET responses.
    reuse = [c for c in cases if c["id"] in ("health", "plaintext", "json")]
    conn = http.client.HTTPConnection(args.host, args.port, timeout=args.timeout)
    try:
        for case in reuse:
            try:
                verify(case, request(case["method"], case["path"], conn=conn))
                if conn.sock is None:
                    raise AssertionError("keep-alive socket closed")
                passed += 1
            except Exception as exc:
                failures.append(f"keepalive/{case['id']}: {exc}")
                break
    finally:
        conn.close()

    # Concurrency is a correctness smoke test, not an RPS measurement.
    plaintext = next(c for c in cases if c["id"] == "plaintext")
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        future_results = list(pool.map(lambda _: request("GET", "/plaintext"), range(24)))
    for index, response in enumerate(future_results):
        try:
            verify(plaintext, response)
            passed += 1
        except Exception as exc:
            failures.append(f"concurrent/{index}: {exc}")
    return passed, failures


try:
    cases = load_cases()["scenarios"]
    if command:
        env = dict(os.environ, HOST=args.host, PORT=str(args.port))
        proc = subprocess.Popen(command, env=env, stdout=subprocess.DEVNULL)
    wait_for_server()
    checks, failures = run(cases)
except Exception as exc:
    checks, failures = 0, [str(exc)]
finally:
    if proc is not None:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=8)

report = {"schema_version": 1, "target": args.target, "passed": not failures,
          "checks": checks, "failures": failures, "benchmarks_run": False}
result_schema = json.loads((CONTRACTS / "result.schema.json").read_text())
Draft202012Validator(result_schema).validate(report)
print(json.dumps(report))
if failures:
    sys.exit(1)
