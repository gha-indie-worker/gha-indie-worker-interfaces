// Deno native HTTP server; no NPM imports or framework middleware.
const host = Deno.env.get("HOST") || "127.0.0.1";
const port = Number(Deno.env.get("PORT") ?? "8080");
if (!Number.isInteger(port) || port < 1 || port > 65535) {
  console.error("PORT must be an integer from 1 to 65535");
  Deno.exit(2);
}
const MAX_BODY = 1024 * 1024;
const encoder = new TextEncoder();
const HEALTH = encoder.encode('{"status":"ok"}');
const TEXT = encoder.encode("hello, world!");
const JSON_BODY = encoder.encode('{"message":"hello, world!"}');

function reply(status, type, body) {
  return new Response(body, {
    status,
    headers: {
      "content-type": type,
      "content-length": String(body.byteLength),
    },
  });
}

async function readBody(request) {
  if (!request.body) return new Uint8Array(0);
  const reader = request.body.getReader();
  const parts = [];
  let size = 0;
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.byteLength;
      if (size > MAX_BODY) {
        await reader.cancel().catch(() => {});
        return null;
      }
      parts.push(value);
    }
  } finally {
    reader.releaseLock();
  }
  const output = new Uint8Array(size);
  let offset = 0;
  for (const part of parts) {
    output.set(part, offset);
    offset += part.byteLength;
  }
  return output;
}

const controller = new AbortController();
for (const signal of ["SIGINT", "SIGTERM"]) {
  Deno.addSignalListener(signal, () => controller.abort());
}
Deno.serve({
  hostname: host,
  port,
  signal: controller.signal,
  onListen: ({ hostname, port }) => console.error(`ready deno http://${hostname}:${port}`),
}, async (request) => {
  const path = new URL(request.url).pathname;
  if (request.method === "GET" && path === "/health") {
    return reply(200, "application/json", HEALTH);
  }
  if (request.method === "GET" && path === "/plaintext") {
    return reply(200, "text/plain; charset=utf-8", TEXT);
  }
  if (request.method === "GET" && path === "/json") {
    return reply(200, "application/json", JSON_BODY);
  }
  if (request.method === "POST" && path === "/echo") {
    try {
      const body = await readBody(request);
      if (body === null) {
        return reply(413, "text/plain; charset=utf-8", encoder.encode("Payload Too Large"));
      }
      return reply(200, "application/octet-stream", body);
    } catch {
      return reply(400, "text/plain; charset=utf-8", encoder.encode("Bad Request"));
    }
  }
  return reply(404, "text/plain; charset=utf-8", encoder.encode("Not Found"));
});
