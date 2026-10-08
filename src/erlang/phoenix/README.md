# Phoenix HTTP benchmark server

Phoenix **1.8.15** (Elixir/BEAM), Bandit **1.12.5**, without Ecto, LiveView, assets, sessions, telemetry instrumentation, or persistence.

```sh
cd src/erlang/phoenix
mix local.hex --force
mix deps.get
mix compile
cd ../../..
python3 scripts/conformance.py -- bash src/erlang/phoenix/run.sh
```

Mix creates `mix.lock`, which should be committed after a successful build for full dependency reproducibility. This is an **Elixir** web framework running on Erlang/OTP and is only filed under `src/erlang` to preserve the repository's requested layout.

Routes: `GET /health`, `GET /plaintext`, `GET /json`, `POST /echo`; a fallback returns 404. Bodies are buffered as binary up to 1 MiB, without conversion through text encodings. Defaults `HOST=127.0.0.1`, `PORT=8080`. No benchmark rankings until live conformance passes.
