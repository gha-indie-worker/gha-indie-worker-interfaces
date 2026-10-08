// Node.js core HTTP benchmark: no framework middleware or request logging.
import http from "node:http";

const port = Number(process.env.PORT ?? "8080");
if (!Number.isSafeInteger(port) || port < 1 || port > 65535) {
  console.error("PORT must be an integer from 1 to 65535");
  process.exit(2);
}
const host = process.env.HOST || "127.0.0.1";
const MAX_BODY = 1024 * 1024;
const text = Buffer.from("hello, world!", "utf8");
const health = Buffer.from('{"status":"ok"}', "utf8");
const json = Buffer.from('{"message":"hello, world!"}', "utf8");

function send(res, status, type, body) {
  res.writeHead(status, {
    "Content-Type": type,
    "Content-Length": body.length,
  });
  res.end(body);
}

const server = http.createServer(async (req, res) => {
  if (req.method === "GET" && req.url === "/health") {
    send(res, 200, "application/json", health);
  } else if (req.method === "GET" && req.url === "/plaintext") {
    send(res, 200, "text/plain; charset=utf-8", text);
  } else if (req.method === "GET" && req.url === "/json") {
    send(res, 200, "application/json", json);
  } else if (req.method === "POST" && req.url === "/echo") {
    // Buffer the body like the Gin/Axum candidates; enforce the same 1 MiB limit.
    const chunks = [];
    let size = 0;
    try {
      for await (const chunk of req) {
        size += chunk.length;
        if (size > MAX_BODY) {
          send(res, 413, "text/plain; charset=utf-8", Buffer.from("Payload Too Large"));
          return;
        }
        chunks.push(chunk);
      }
      send(res, 200, "application/octet-stream", Buffer.concat(chunks, size));
    } catch (error) {
      if (!res.headersSent && !res.destroyed) {
        send(res, 400, "text/plain; charset=utf-8", Buffer.from("Bad Request"));
      }
    }
  } else {
    send(res, 404, "text/plain; charset=utf-8", Buffer.from("Not Found"));
  }
});
server.requestTimeout = 30_000;
server.headersTimeout = 30_000;
server.listen(port, host, () => {
  console.error(`ready nodejs http://${host}:${port}`);
});
for (const signal of ["SIGINT", "SIGTERM"]) {
  process.on(signal, () => server.close(() => process.exit(0)));
}
