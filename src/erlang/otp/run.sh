#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
exec erl -noshell -pa _build/default/lib/*/ebin -eval 'benchmark_otp:start().'
