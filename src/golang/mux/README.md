# Gorilla/mux HTTP benchmark

Uses Go net/http with `github.com/gorilla/mux` v1.8.1 as a router.
Build: `cd src/golang/mux && go mod tidy && go build -o server .`.
Run from repository root: `src/golang/mux/server`.
Check: `python3 scripts/conformance.py --target golang-mux -- ./src/golang/mux/server`.

This measures net/http + Gorilla/mux, not Gorilla/mux as a standalone HTTP server. Body buffering and the 1 MiB echo limit match Gin.
