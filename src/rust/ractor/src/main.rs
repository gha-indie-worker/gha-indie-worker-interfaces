// Ractor's actor mailbox does the benchmark work; Axum only implements HTTP transport.
// This is an actor + HTTP integration target, not an independent HTTP framework.
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{header, StatusCode},
    routing::{get, post},
    Router,
};
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use std::{env, net::SocketAddr, time::Duration};

const MAX_BODY: usize = 1024 * 1024;

#[derive(Clone, Copy)]
enum Endpoint {
    Health,
    Plaintext,
    Json,
    Echo,
}

struct ActorResponse {
    content_type: &'static str,
    body: Bytes,
}

enum Message {
    Process(Endpoint, Bytes, RpcReplyPort<ActorResponse>),
}

struct BenchmarkActor;

impl Actor for BenchmarkActor {
    type Msg = Message;
    type State = ();
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: (),
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        let Message::Process(endpoint, payload, reply) = msg;
        let value = match endpoint {
            Endpoint::Health => ActorResponse {
                content_type: "application/json",
                body: Bytes::from_static(br#"{"status":"ok"}"#),
            },
            Endpoint::Plaintext => ActorResponse {
                content_type: "text/plain; charset=utf-8",
                body: Bytes::from_static(b"hello, world!"),
            },
            Endpoint::Json => ActorResponse {
                content_type: "application/json",
                body: Bytes::from_static(br#"{"message":"hello, world!"}"#),
            },
            Endpoint::Echo => ActorResponse {
                content_type: "application/octet-stream",
                body: payload,
            },
        };
        let _ = reply.send(value);
        Ok(())
    }
}

#[derive(Clone)]
struct AppState {
    actor: ActorRef<Message>,
}

type HttpReply = (StatusCode, [(header::HeaderName, &'static str); 1], Bytes);

async fn dispatch(state: AppState, endpoint: Endpoint, input: Bytes) -> HttpReply {
    let result = state
        .actor
        .call(
            move |reply| Message::Process(endpoint, input, reply),
            Some(Duration::from_secs(30)),
        )
        .await;
    match result {
        Ok(response) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, response.content_type)],
            response.body,
        ),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            Bytes::from_static(b"Actor unavailable"),
        ),
    }
}
async fn health(State(state): State<AppState>) -> HttpReply {
    dispatch(state, Endpoint::Health, Bytes::new()).await
}
async fn plaintext(State(state): State<AppState>) -> HttpReply {
    dispatch(state, Endpoint::Plaintext, Bytes::new()).await
}
async fn json(State(state): State<AppState>) -> HttpReply {
    dispatch(state, Endpoint::Json, Bytes::new()).await
}
async fn echo(State(state): State<AppState>, body: Bytes) -> HttpReply {
    dispatch(state, Endpoint::Echo, body).await
}
async fn not_found() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not Found")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port: u16 = env::var("PORT").unwrap_or_else(|_| "8080".into()).parse()?;
    if port == 0 { return Err("PORT must be 1..65535".into()); }
    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let (actor, actor_task) = Actor::spawn(None, BenchmarkActor, ()).await?;
    let app = Router::new()
        .route("/health", get(health))
        .route("/plaintext", get(plaintext))
        .route("/json", get(json))
        .route("/echo", post(echo))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .fallback(not_found)
        .with_state(AppState { actor: actor.clone() });
    let listener = tokio::net::TcpListener::bind(addr).await?;
    eprintln!("ready ractor-axum http://{addr}");
    let result = axum::serve(listener, app)
        .with_graceful_shutdown(async {
            if let Err(error) = tokio::signal::ctrl_c().await {
                eprintln!("shutdown signal error: {error}");
            }
        })
        .await;
    actor.stop(None);
    actor_task.await?;
    result?;
    Ok(())
}
