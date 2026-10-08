defmodule BenchmarkPhoenix.Router do
  @moduledoc false
  use Phoenix.Router

  get "/health", BenchmarkPhoenix.Handler, :health
  get "/plaintext", BenchmarkPhoenix.Handler, :plaintext
  get "/json", BenchmarkPhoenix.Handler, :json
  post "/echo", BenchmarkPhoenix.Handler, :echo
  match :*, "/*path", BenchmarkPhoenix.Handler, :not_found
end
