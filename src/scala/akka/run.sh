#!/usr/bin/env bash
set -euo pipefail
dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
if [[ ! -f "$dir/target/classpath.txt" || ! -d "$dir/target/classes" ]]; then
  echo "Build first: (cd $dir && mvn -q -DskipTests package dependency:build-classpath -Dmdep.outputFile=target/classpath.txt)" >&2
  exit 66
fi
exec java -cp "$dir/target/classes:$(cat "$dir/target/classpath.txt")" bench.akka.Server
