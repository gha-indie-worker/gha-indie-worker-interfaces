# Akka HTTP / Scala benchmark server

Scala 2.13.17, Akka HTTP **10.2.10**, Akka Stream **2.6.20**, Java 17+, and Maven. The deliberately older Apache-2-licensed Akka artifacts are available from Maven Central without Akka's newer secure tokenized repository. This is a named **legacy Akka HTTP** comparison, not a current Akka HTTP release; future comparisons may add a separate modern commercially licensed target.

```sh
cd src/scala/akka
mvn -q -DskipTests package dependency:build-classpath -Dmdep.outputFile=target/classpath.txt
cd ../../..
python3 scripts/conformance.py -- bash src/scala/akka/run.sh
```

Implements the four fixed HTTP routes (GET health/plaintext/json and POST binary echo). Request entity is buffered with a 1 MiB size limit, no custom middleware; HOST/PORT default to 127.0.0.1:8080. Results must not be published until 35/35 conformance succeeds.

**Licensing and provenance matter:** pin the full Maven dependency graph and report this old Akka version alongside measurements.
