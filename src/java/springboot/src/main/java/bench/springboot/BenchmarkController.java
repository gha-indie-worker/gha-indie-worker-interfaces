package bench.springboot;

import jakarta.servlet.http.HttpServletRequest;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import org.springframework.http.HttpHeaders;
import org.springframework.http.HttpStatus;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RestController;

@RestController
public final class BenchmarkController {
  private static final int MAX_BODY = 1024 * 1024;
  private static final byte[] HEALTH = ascii("{\"status\":\"ok\"}");
  private static final byte[] TEXT = ascii("hello, world!");
  private static final byte[] JSON = ascii("{\"message\":\"hello, world!\"}");

  private static byte[] ascii(String value) {
    return value.getBytes(StandardCharsets.UTF_8);
  }

  private static ResponseEntity<byte[]> respond(int status, String mime, byte[] body) {
    HttpHeaders headers = new HttpHeaders();
    headers.set(HttpHeaders.CONTENT_TYPE, mime);
    headers.setContentLength(body.length);
    return new ResponseEntity<>(body, headers, HttpStatus.valueOf(status));
  }

  @GetMapping("/health")
  public ResponseEntity<byte[]> health() {
    return respond(200, "application/json", HEALTH);
  }

  @GetMapping("/plaintext")
  public ResponseEntity<byte[]> plaintext() {
    return respond(200, "text/plain; charset=utf-8", TEXT);
  }

  @GetMapping("/json")
  public ResponseEntity<byte[]> json() {
    return respond(200, "application/json", JSON);
  }

  @PostMapping("/echo")
  public ResponseEntity<byte[]> echo(HttpServletRequest request) {
    // Explicit bounded buffering, independent of Content-Length and Content-Type.
    try {
      byte[] buffer = new byte[64 * 1024];
      ByteArrayOutputStream output = new ByteArrayOutputStream();
      int count;
      while ((count = request.getInputStream().read(buffer)) != -1) {
        if (output.size() + count > MAX_BODY) {
          return respond(413, "text/plain; charset=utf-8", ascii("Payload Too Large"));
        }
        output.write(buffer, 0, count);
      }
      return respond(200, "application/octet-stream", output.toByteArray());
    } catch (IOException exception) {
      return respond(400, "text/plain; charset=utf-8", ascii("Bad Request"));
    }
  }
}
