package main

import (
    "context"
    "errors"
    "fmt"
    "io"
    "log"
    "net/http"
    "os"
    "os/signal"
    "strconv"
    "syscall"
    "time"

    "github.com/gorilla/mux"
)

const maxBody = 1024 * 1024

func reply(w http.ResponseWriter, status int, contentType string, body []byte) {
    w.Header().Set("Content-Type", contentType)
    w.Header().Set("Content-Length", strconv.Itoa(len(body)))
    w.WriteHeader(status)
    _, _ = w.Write(body)
}

func main() {
    port := 8080
    if value := os.Getenv("PORT"); value != "" {
        parsed, err := strconv.Atoi(value)
        if err != nil || parsed < 1 || parsed > 65535 {
            log.Fatal("PORT must be an integer from 1 to 65535")
        }
        port = parsed
    }
    host := os.Getenv("HOST")
    if host == "" { host = "127.0.0.1" }

    router := mux.NewRouter()
    router.HandleFunc("/health", func(w http.ResponseWriter, _ *http.Request) {
        reply(w, 200, "application/json", []byte(`{"status":"ok"}`))
    }).Methods(http.MethodGet)
    router.HandleFunc("/plaintext", func(w http.ResponseWriter, _ *http.Request) {
        reply(w, 200, "text/plain; charset=utf-8", []byte("hello, world!"))
    }).Methods(http.MethodGet)
    router.HandleFunc("/json", func(w http.ResponseWriter, _ *http.Request) {
        reply(w, 200, "application/json", []byte(`{"message":"hello, world!"}`))
    }).Methods(http.MethodGet)
    router.HandleFunc("/echo", func(w http.ResponseWriter, r *http.Request) {
        defer r.Body.Close()
        body, err := io.ReadAll(io.LimitReader(r.Body, maxBody+1))
        if err != nil {
            reply(w, 400, "text/plain; charset=utf-8", []byte("Bad Request"))
        } else if len(body) > maxBody {
            reply(w, 413, "text/plain; charset=utf-8", []byte("Payload Too Large"))
        } else {
            reply(w, 200, "application/octet-stream", body)
        }
    }).Methods(http.MethodPost)
    router.NotFoundHandler = http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
        reply(w, 404, "text/plain; charset=utf-8", []byte("Not Found"))
    })

    server := &http.Server{
        Addr: fmt.Sprintf("%s:%d", host, port),
        Handler: router,
        ReadHeaderTimeout: 30 * time.Second,
    }
    ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
    defer stop()
    go func() {
        <-ctx.Done()
        shutdownCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
        defer cancel()
        if err := server.Shutdown(shutdownCtx); err != nil {
            log.Printf("shutdown error: %v", err)
        }
    }()
    log.Printf("ready mux http://%s:%d", host, port)
    if err := server.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
        log.Fatal(err)
    }
}
