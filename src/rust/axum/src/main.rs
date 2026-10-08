use axum::{
    body::{Body, Bytes},
    extract::DefaultBodyLimit,
    http::{header, HeaderValue, StatusCode},
    routing::{get, post},
    Router,
};
use std::{env, net::SocketAddr};

async fn health() -> ([(header::HeaderName, &'static str); 1], &'static str) {
    ([(header::CONTENT_TYPE, "application/json")], r#"{"status":"ok"}"#)
}
async fn plaintext() -> ([(header::HeaderName, &'static str); 1], &'static str) {
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], "hello, world!")
}
async fn json() -> ([(header::HeaderName, &'static str); 1], &'static str) {
    ([(header::CONTENT_TYPE, "application/json")], r#"{"message":"hello, world!"}"#)
}
async fn echo(body: Bytes) -> ([(header::HeaderName, &'static str); 1], Bytes) {
    ([(header::CONTENT_TYPE, "application/octet-stream")], body)
}
async fn not_found() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not Found")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = env::var("PORT").unwrap_or_else(|_| "8080".to_string()).parse()?;
    if port == 0 { return Err("PORT must be 1..65535".into()); }
    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let app = Router::new()
        .route("/health", get(health))
        .route("/plaintext", get(plaintext))
        .route("/json", get(json))
        .route("/echo", post(echo))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .fallback(not_found);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    eprintln!("ready axum http://{addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            if let Err(error) = tokio::signal::ctrl_c().await {
                eprintln!("shutdown signal error: {error}");
            }
        })
        .await?;
    Ok(())
}
