defmodule BenchmarkPhoenix.Application do
  @moduledoc false
  use Application

  @impl true
  def start(_type, _args) do
    Supervisor.start_link([BenchmarkPhoenix.Endpoint],
      strategy: :one_for_one, name: BenchmarkPhoenix.Supervisor)
  end
end
