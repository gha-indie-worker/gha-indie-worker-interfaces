#!/usr/bin/env python3
"""Validate the fixture corpus against the HAND-AUTHORED JSON Schema authority.

This is one lane of the parity gate, not the gate itself. `scripts/parity.mjs` runs
this script for the JSON Schema side and runs the same corpus against the schema
TypeSpec emits, then requires the two verdicts to agree fixture-for-fixture. A
negative fixture that only one authority rejects is exactly the discrepancy the
two-authority model exists to surface, so it fails closed there.

Run standalone:

    python3 scripts/check_fixtures.py            # human output, exit 1 on any failure
    python3 scripts/check_fixtures.py --json     # machine output for parity.mjs
    python3 scripts/check_fixtures.py --documents target/parity/lane.json --lane typespec

`--documents` points every document key in the manifest at a different schema file
(parity.mjs points them all at the single bundle `tsp` emits) so the SAME corpus is
run against the other authority by the SAME code. Two verdict maps produced this way
are directly comparable; two different validators would not be.

Requires `jsonschema` (2020-12). No network, no npm — this lane runs anywhere
Python does, which is why it is the lane that gates a container without a
TypeSpec toolchain.
"""
from __future__ import annotations

import argparse
import base64
import binascii
import json
import re
import sys
from pathlib import Path

try:
    from jsonschema import Draft202012Validator, FormatChecker
except ImportError:  # pragma: no cover - environment problem, not a contract problem
    sys.stderr.write("check_fixtures: `jsonschema` is not installed (pip install jsonschema)\n")
    raise SystemExit(2)

ROOT = Path(__file__).resolve().parent.parent
FIXTURES = ROOT / "contracts" / "fixtures"

# --- formats -----------------------------------------------------------------
# jsonschema ships no checker for `date-time` unless rfc3339-validator is present,
# and none at all for `byte`. Leaving them unchecked would let a negative fixture
# pass by accident, which is worse than having no fixture: it would be evidence of
# agreement that was never tested. So both are checked here explicitly, with the
# same rules ajv is configured with in package.json.

_RFC3339 = re.compile(
    r"^\d{4}-\d{2}-\d{2}[Tt]\d{2}:\d{2}:\d{2}(\.\d+)?([Zz]|[+-]\d{2}:\d{2})$"
)

FORMATS = FormatChecker()


@FORMATS.checks("date-time", raises=ValueError)
def _date_time(value: object) -> bool:
    if not isinstance(value, str):
        return True
    if not _RFC3339.match(value):
        return False
    import datetime as _dt

    _dt.datetime.fromisoformat(value.replace("Z", "+00:00").replace("z", "+00:00"))
    return True


@FORMATS.checks("byte", raises=(binascii.Error, ValueError))
def _byte(value: object) -> bool:
    if not isinstance(value, str):
        return True
    if re.search(r"[^A-Za-z0-9+/=]", value):
        return False
    base64.b64decode(value, validate=True)
    return True


for _name in ("uuid", "date", "regex"):
    if _name in Draft202012Validator.FORMAT_CHECKER.checkers:
        FORMATS.checkers[_name] = Draft202012Validator.FORMAT_CHECKER.checkers[_name]


# --- corpus ------------------------------------------------------------------
def load_manifest() -> dict:
    return json.loads((FIXTURES / "index.json").read_text())


def load_documents(manifest: dict, override: Path | None = None) -> dict[str, dict]:
    """Manifest key -> parsed schema document.

    With `override`, every key is pointed at the file that JSON names for it (or at
    its single `"*"` entry), which is how the TypeSpec-emitted bundle is substituted
    without touching the fixture corpus or the manifest.
    """
    if override is None:
        return {key: json.loads((ROOT / rel).read_text()) for key, rel in manifest["documents"].items()}
    mapping = json.loads(override.read_text())
    cache: dict[str, dict] = {}
    documents: dict[str, dict] = {}
    for key in manifest["documents"]:
        rel = mapping.get(key, mapping.get("*"))
        if rel is None:
            continue
        if rel not in cache:
            cache[rel] = json.loads((ROOT / rel).read_text() if not Path(rel).is_absolute() else Path(rel).read_text())
        documents[key] = cache[rel]
    return documents


def validator_for(document: dict, model: str) -> Draft202012Validator:
    """A validator rooted at `#/$defs/<model>` that can still resolve sibling $refs."""
    schema = {"$schema": document["$schema"], "$ref": f"#/$defs/{model}", "$defs": document["$defs"]}
    return Draft202012Validator(schema, format_checker=FORMATS)


def fixture_files(kind: str, model: str) -> list[Path]:
    directory = FIXTURES / kind / model
    return sorted(directory.glob("*.json")) if directory.is_dir() else []


def check(manifest: dict, documents: dict[str, dict]) -> tuple[list[dict], dict]:
    findings: list[dict] = []
    results: dict[str, dict[str, str]] = {}
    stats = {"positive_passed": 0, "positive_failed": 0, "negative_rejected": 0, "negative_accepted": 0}

    declared = set(manifest["models"])
    union_members = {m for members in manifest.get("unionMembers", {}).values() for m in members}
    for key, document in documents.items():
        for name, sub in document["$defs"].items():
            if "enum" in sub or name in union_members or name in declared:
                continue
            findings.append(
                {
                    "kind": "fixture-coverage",
                    "detail": f"{key}.schema.json $defs.{name} is a model with no entry in contracts/fixtures/index.json",
                }
            )

    for model, doc_key in manifest["models"].items():
        document = documents.get(doc_key)
        if document is None:
            findings.append({"kind": "fixture-manifest", "detail": f"{model} names unknown document {doc_key!r}"})
            continue
        if model not in document["$defs"]:
            findings.append({"kind": "fixture-manifest", "detail": f"{model} is not a $defs of {doc_key}"})
            continue
        validator = validator_for(document, model)
        positives, negatives = fixture_files("valid", model), fixture_files("invalid", model)
        if not positives:
            findings.append({"kind": "fixture-coverage", "detail": f"{model} has no positive fixture"})
        if not negatives:
            findings.append({"kind": "fixture-coverage", "detail": f"{model} has no negative fixture"})

        for path in positives:
            rel = str(path.relative_to(ROOT))
            errors = sorted(validator.iter_errors(json.loads(path.read_text())), key=str)
            if errors:
                stats["positive_failed"] += 1
                results.setdefault(model, {})[rel] = "rejected"
                findings.append(
                    {
                        "kind": "fixture-positive",
                        "detail": f"{rel} must validate against {doc_key}#/$defs/{model} but does not: "
                        + "; ".join(f"{'/'.join(map(str, e.absolute_path)) or '<root>'}: {e.message}" for e in errors[:3]),
                    }
                )
            else:
                stats["positive_passed"] += 1
                results.setdefault(model, {})[rel] = "accepted"

        for path in negatives:
            rel = str(path.relative_to(ROOT))
            errors = list(validator.iter_errors(json.loads(path.read_text())))
            if errors:
                stats["negative_rejected"] += 1
                results.setdefault(model, {})[rel] = "rejected"
            else:
                stats["negative_accepted"] += 1
                results.setdefault(model, {})[rel] = "accepted"
                findings.append(
                    {
                        "kind": "fixture-negative",
                        "detail": f"{rel} must be REJECTED by {doc_key}#/$defs/{model} and was accepted — "
                        "either the fixture is not actually invalid, or the schema is missing a constraint",
                    }
                )

    return findings, {"stats": stats, "results": results}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="emit machine-readable output for scripts/parity.mjs")
    parser.add_argument("--documents", type=Path, default=None, help='JSON map {"<docKey>"|"*": "<schema path>"} substituting the schema under test')
    parser.add_argument("--lane", default="json-schema", help="lane name recorded in --json output")
    args = parser.parse_args()

    manifest = load_manifest()
    documents = load_documents(manifest, args.documents)
    findings, detail = check(manifest, documents)
    stats = detail["stats"]

    if args.json:
        json.dump({"lane": args.lane, "findings": findings, **detail}, sys.stdout, indent=2, sort_keys=True)
        sys.stdout.write("\n")
    else:
        print(
            f"[fixtures] {args.lane}: {stats['positive_passed']}/"
            f"{stats['positive_passed'] + stats['positive_failed']} positives accepted, "
            f"{stats['negative_rejected']}/{stats['negative_rejected'] + stats['negative_accepted']} negatives rejected "
            f"across {len(manifest['models'])} models"
        )
        for finding in findings:
            print(f"[fixtures]   ✗ {finding['kind']}: {finding['detail']}")
        print(f"[fixtures] {'PASSED' if not findings else 'STOPPED_FOR_EVALUATION'}")
    return 1 if findings else 0


if __name__ == "__main__":
    raise SystemExit(main())
