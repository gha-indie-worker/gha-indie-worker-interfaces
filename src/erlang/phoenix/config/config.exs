import Config

config :benchmark_phoenix, BenchmarkPhoenix.Endpoint,
  adapter: Bandit.PhoenixAdapter,
  server: true,
  secret_key_base: String.duplicate("b", 64),
  http: [ip: {127, 0, 0, 1}, port: 8080]

config :logger, level: :warning
