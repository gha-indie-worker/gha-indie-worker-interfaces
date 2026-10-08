#!/usr/bin/env python3
"""Static source equivalence check; not proof of runnable Spin code."""
from pathlib import Path
base = Path(__file__).resolve().parents[1] / "src/oreslang/spin"
shared = (base / "server_shared.ores").read_text()
isolated = (base / "server_isolated.ores").read_text()
normalized = shared.replace("(shared actor per request)", "(isolated actor per request)")
normalized = normalized.replace("define actor BenchmarkHandler as", "define isoactor BenchmarkHandler as")
assert normalized == isolated, "Spin actor variants do not have identical handler logic"
for route in ('"/health"', '"/plaintext"', '"/json"', '"/echo"'):
    assert route in shared, route
print("Spin source parity OK (not a compiler/runtime test)")
