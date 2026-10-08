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

    "github.com/gin-gonic/gin"
)

const maxBody = 1024 * 1024

func main() {
    port := 8080
    if val := os.Getenv("PORT"); val != "" {
        parsed, err := strconv.Atoi(val)
        if err != nil || parsed < 1 || parsed > 65535 {
            log.Fatal("PORT must be an integer from 1 to 65535")
        }
        port = parsed
    }
    host := os.Getenv("HOST")
    if host == "" { host = "127.0.0.1" }
    gin.SetMode(gin.ReleaseMode)
    router := gin.New() // No default Logger or Recovery middleware.
    router.GET("/health", func(c *gin.Context) {
        c.Data(http.StatusOK, "application/json", []byte(`{"status":"ok"}`))
    })
    router.GET("/plaintext", func(c *gin.Context) {
        c.Data(http.StatusOK, "text/plain; charset=utf-8", []byte("hello, world!"))
    })
    router.GET("/json", func(c *gin.Context) {
        c.Data(http.StatusOK, "application/json", []byte(`{"message":"hello, world!"}`))
    })
    router.POST("/echo", func(c *gin.Context) {
        body, err := io.ReadAll(io.LimitReader(c.Request.Body, maxBody+1))
        if err != nil {
            c.Data(http.StatusBadRequest, "text/plain; charset=utf-8", []byte("Bad Request"))
            return
        }
        if len(body) > maxBody {
            c.Data(http.StatusRequestEntityTooLarge, "text/plain; charset=utf-8", []byte("Payload Too Large"))
            return
        }
        c.Data(http.StatusOK, "application/octet-stream", body)
    })
    router.NoRoute(func(c *gin.Context) {
        c.Data(http.StatusNotFound, "text/plain; charset=utf-8", []byte("Not Found"))
    })
    server := &http.Server{
        Addr: fmt.Sprintf("%s:%d", host, port),
        Handler: router,
        ReadHeaderTimeout: 30 * time.Second,
    }
    stopCtx, cancel := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
    defer cancel()
    go func() {
        <-stopCtx.Done()
        ctx, stop := context.WithTimeout(context.Background(), 5*time.Second)
        defer stop()
        if err := server.Shutdown(ctx); err != nil { log.Printf("shutdown error: %v", err) }
    }()
    log.Printf("ready gin http://%s:%d", host, port)
    if err := server.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
        log.Fatal(err)
    }
}
