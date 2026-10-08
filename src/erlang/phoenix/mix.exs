defmodule BenchmarkPhoenix.MixProject do
  use Mix.Project

  def project do
    [
      app: :benchmark_phoenix,
      version: "0.1.0",
      elixir: "~> 1.18",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  def application do
    [
      mod: {BenchmarkPhoenix.Application, []},
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:phoenix, "== 1.8.15"},
      {:bandit, "== 1.12.5"}
    ]
  end
end
