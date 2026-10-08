defmodule BenchmarkPhoenix.Endpoint do
  @moduledoc false
  use Phoenix.Endpoint, otp_app: :benchmark_phoenix

  plug BenchmarkPhoenix.Router
end
