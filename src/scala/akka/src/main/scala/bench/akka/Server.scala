package bench.akka

import akka.actor.ActorSystem
import akka.http.scaladsl.Http
import akka.http.scaladsl.model.{ContentTypes, HttpEntity, HttpResponse, StatusCodes}
import akka.http.scaladsl.server.{Route, Directives}
import akka.stream.SystemMaterializer

import scala.concurrent.Await
import scala.concurrent.duration._

/** Akka HTTP standalone server. Body is fully buffered to match other candidates. */
object Server extends Directives {
  def main(args: Array[String]): Unit = {
    val host = sys.env.getOrElse("HOST", "127.0.0.1")
    val port = try {
      val p = sys.env.getOrElse("PORT", "8080").toInt
      require(p > 0 && p <= 65535)
      p
    } catch {
      case _: Exception =>
        System.err.println("PORT must be an integer from 1 to 65535")
        sys.exit(2)
    }

    implicit val system: ActorSystem = ActorSystem("oreslang-akka-benchmark")
    implicit val materializer: akka.stream.Materializer = SystemMaterializer(system).materializer

    val route: Route = concat(
      path("health") {
        get {
          complete(HttpResponse(entity =
            HttpEntity(ContentTypes.`application/json`, """{"status":"ok"}""")))
        }
      },
      path("plaintext") {
        get {
          complete(HttpResponse(entity =
            HttpEntity(ContentTypes.`text/plain(UTF-8)`, "hello, world!")))
        }
      },
      path("json") {
        get {
          complete(HttpResponse(entity =
            HttpEntity(ContentTypes.`application/json`, """{"message":"hello, world!"}""")))
        }
      },
      path("echo") {
        post {
          extractRequestEntity { incoming =>
            onSuccess(incoming.withSizeLimit(1024 * 1024).toStrict(30.seconds)) { strict =>
              complete(HttpResponse(
                entity = HttpEntity(ContentTypes.`application/octet-stream`, strict.data)
              ))
            }
          }
        }
      }
    )

    val binding = Await.result(Http().newServerAt(host, port).bind(route), 90.seconds)
    System.err.println(s"ready akka http://$host:$port")
    sys.addShutdownHook {
      Await.ready(binding.unbind(), 10.seconds)
      system.terminate()
    }
    Await.result(system.whenTerminated, Duration.Inf)
  }
}
