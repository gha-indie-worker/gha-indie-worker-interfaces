defmodule BenchmarkPhoenix.Handler do
  @moduledoc false
  import Plug.Conn

  @max_body 1_048_576
  @read_size 65_536

  def init(action), do: action

  def call(conn, :health) do
    conn |> put_resp_header("content-type", "application/json")
    |> send_resp(200, ~s({"status":"ok"}))
  end

  def call(conn, :plaintext) do
    conn |> put_resp_header("content-type", "text/plain; charset=utf-8")
    |> send_resp(200, "hello, world!")
  end

  def call(conn, :json) do
    conn |> put_resp_header("content-type", "application/json")
    |> send_resp(200, ~s({"message":"hello, world!"}))
  end

  def call(conn, :echo) do
    case read_entire_body(conn, [], 0) do
      {:ok, conn, body} ->
        conn |> put_resp_header("content-type", "application/octet-stream")
        |> send_resp(200, body)
      {:too_large, conn} ->
        conn |> put_resp_header("content-type", "text/plain; charset=utf-8")
        |> send_resp(413, "Payload Too Large")
      {:error, conn} ->
        conn |> put_resp_header("content-type", "text/plain; charset=utf-8")
        |> send_resp(400, "Bad Request")
    end
  end

  def call(conn, :not_found) do
    conn |> put_resp_header("content-type", "text/plain; charset=utf-8")
    |> send_resp(404, "Not Found")
  end

  # Explicitly preserve binary payloads, including NUL/non-UTF8 and 1 MiB.
  defp read_entire_body(conn, chunks, total) do
    case read_body(conn, length: @read_size, read_length: @read_size, read_timeout: 30_000) do
      {:ok, chunk, conn} ->
        if total + byte_size(chunk) > @max_body do
          {:too_large, conn}
        else
          {:ok, conn, IO.iodata_to_binary(Enum.reverse([chunk | chunks]))}
        end
      {:more, chunk, conn} ->
        if total + byte_size(chunk) > @max_body do
          {:too_large, conn}
        else
          read_entire_body(conn, [chunk | chunks], total + byte_size(chunk))
        end
      {:error, _reason} ->
        {:error, conn}
    end
  end
end
