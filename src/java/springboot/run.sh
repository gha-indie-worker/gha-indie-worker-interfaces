#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
exec java -jar target/oreslang-benchmark-springboot-0.1.0.jar
