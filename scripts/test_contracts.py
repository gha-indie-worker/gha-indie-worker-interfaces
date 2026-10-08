#!/usr/bin/env python3
"""Negative regression tests: schema and OpenAPI contract drift must fail CI."""
import copy
import unittest

from check_contracts import REQUIRED, load_cases, check_openapi


class ContractTests(unittest.TestCase):
    def test_load_cases(self):
        cases = load_cases()
        self.assertEqual(len(cases["scenarios"]), 8)

    def test_required_routes(self):
        self.assertEqual(set(REQUIRED), {
            ("GET", "/health"), ("GET", "/plaintext"),
            ("GET", "/json"), ("POST", "/echo")})

    def test_invalid_openapi_missing_method(self):
        doc = {"paths": {}}
        with self.assertRaises(ValueError):
            check_openapi(doc)

    def test_schema_rejects_bad_status(self):
        from jsonschema import Draft202012Validator, ValidationError
        from check_contracts import CONTRACTS
        import json
        schema = json.loads((CONTRACTS / "cases.schema.json").read_text())
        cases = copy.deepcopy(load_cases())
        cases["scenarios"][0]["expected"]["status"] = "200"
        with self.assertRaises(ValidationError):
            Draft202012Validator(schema).validate(cases)

    def test_json_contract_fixture(self):
        cases = {c["id"]: c for c in load_cases()["scenarios"]}
        self.assertEqual(cases["health"]["expected"]["body"]["equals"], {"status": "ok"})
        self.assertEqual(cases["json"]["expected"]["body"]["equals"], {"message": "hello, world!"})


if __name__ == "__main__":
    unittest.main()
