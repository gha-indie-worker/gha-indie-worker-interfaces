package bench.vertx;

import io.vertx.core.Vertx;
import io.vertx.core.buffer.Buffer;
import io.vertx.core.http.HttpServerOptions;
import io.vertx.ext.web.Router;
import io.vertx.ext.web.RoutingContext;
import io.vertx.ext.web.handler.BodyHandler;

/** Identical fixed-body and binary echo HTTP contract for Vert.x Web. */
public final class Server {
  private static final int MAX_BODY = 1_048_576;

  private Server() { }

  private static void respond(RoutingContext ctx, int status, String mime, String text) {
    ctx.response().setStatusCode(status).putHeader("Content-Type", mime).end(text);
  }

  public static void main(String[] args) {
    final String host = System.getenv().getOrDefault("HOST", "127.0.0.1");
    final int port;
    try {
      port = Integer.parseInt(System.getenv().getOrDefault("PORT", "8080"));
      if (port < 1 || port > 65535) {
        throw new NumberFormatException("outside valid port range");
      }
    } catch (NumberFormatException e) {
      System.err.println("PORT must be an integer from 1 to 65535");
      System.exit(2);
      return;
    }

    Vertx vertx = Vertx.vertx();
    Router router = Router.router(vertx);
    router.get("/health").handler(ctx ->
        respond(ctx, 200, "application/json", "{\"status\":\"ok\"}"));
    router.get("/plaintext").handler(ctx ->
        respond(ctx, 200, "text/plain; charset=utf-8", "hello, world!"));
    router.get("/json").handler(ctx ->
        respond(ctx, 200, "application/json", "{\"message\":\"hello, world!\"}"));
    router.post("/echo")
        .handler(BodyHandler.create().setHandleFileUploads(false).setBodyLimit(MAX_BODY))
        .handler(ctx -> {
          Buffer body = ctx.body() == null ? null : ctx.body().buffer();
          if (body == null) {
            // Vert.x returns a null buffer for an empty request body.
            body = Buffer.buffer();
          }
          ctx.response().setStatusCode(200).putHeader("Content-Type", "application/octet-stream").end(body);
        });
    // No logging/session/cookie middleware. Router preserves HTTP/1.1 keep-alive.
    router.route().handler(ctx ->
        respond(ctx, 404, "text/plain; charset=utf-8", "Not Found"));

    Runtime.getRuntime().addShutdownHook(new Thread(vertx::close));
    vertx.createHttpServer(new HttpServerOptions()
        .setHost(host).setPort(port).setCompressionSupported(false))
        .requestHandler(router)
        .listen()
        .onSuccess(s -> System.err.println("ready vertx http://" + host + ":" + s.actualPort()))
        .onFailure(error -> {
          System.err.println("Vert.x server startup failed: " + error);
          System.exit(1);
        });
  }
}
