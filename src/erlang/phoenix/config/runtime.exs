import Config

host = System.get_env("HOST", "127.0.0.1")
port = case Integer.parse(System.get_env("PORT", "8080")) do
  {num, ""} when num in 1..65_535 -> num
  _ -> raise "PORT must be an integer from 1 to 65535"
end
ip = case :inet.parse_address(String.to_charlist(host)) do
  {:ok, address} -> address
  {:error, _} -> raise "HOST must be a valid IP address"
end

config :benchmark_phoenix, BenchmarkPhoenix.Endpoint,
  http: [ip: ip, port: port],
  url: [host: host, port: port],
  server: true
