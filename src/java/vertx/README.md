# Vert.x Web benchmark server

Uses **Vert.x Web 4.5.34** with Java 17+ and Maven. No logging, JSON encoder, or extra middleware. `POST /echo` buffers at most 1 MiB through Vert.x BodyHandler. Default: `HOST=127.0.0.1`, `PORT=8080`.

```sh
cd src/java/vertx
mvn -q -DskipTests package dependency:build-classpath -Dmdep.outputFile=target/classpath.txt
cd ../../..
python3 scripts/conformance.py -- bash src/java/vertx/run.sh
```

HTTP paths: `GET /health`, `GET /plaintext`, `GET /json`, `POST /echo`. Run benchmarks only after 35/35 correctness checks; do not compare unverified figures.
