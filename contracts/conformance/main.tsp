import "@typespec/http";
using TypeSpec.Http;

@service(#{ title: "Oreslang HTTP benchmark conformance", version: "1.0.0" })
namespace BenchmarkConformance {
  model HealthResponse {
    status: "ok";
  }

  model MessageResponse {
    message: "hello, world!";
  }

  @route("/health")
  @get
  op health(): {
    @statusCode statusCode: 200;
    @header contentType: "application/json";
    @body value: HealthResponse;
  };

  @route("/plaintext")
  @get
  op plaintext(): {
    @statusCode statusCode: 200;
    @header contentType: "text/plain; charset=utf-8";
    @body value: "hello, world!";
  };

  @route("/json")
  @get
  op json(): {
    @statusCode statusCode: 200;
    @header contentType: "application/json";
    @body value: MessageResponse;
  };

  @route("/echo")
  @post
  op echo(@header contentType: "application/octet-stream", @body payload: bytes):
    | {
        @statusCode statusCode: 200;
        @header contentType: "application/octet-stream";
        @body value: bytes;
      }
    | {
        @statusCode statusCode: 413;
        @header contentType: "text/plain; charset=utf-8";
        @body value: string;
      };
}
