# Spring Boot / embedded Tomcat benchmark

The Spring Boot 3.5.6 HTTP benchmark uses `spring-boot-starter-web` (Spring MVC on embedded Tomcat).
Build from `src/java/springboot`: `mvn -B -q -DskipTests package`.
Run from repository root: `bash src/java/springboot/run.sh`.
Check: `python3 scripts/conformance.py --target java-springboot -- bash src/java/springboot/run.sh`.

Set `HOST` and `PORT` to override the default 127.0.0.1:8080. No authentication, templating, Jackson encoding, or custom middleware is involved in the four benchmark handlers. Request bodies are read in bounded chunks and echoed unchanged through 1 MiB.
