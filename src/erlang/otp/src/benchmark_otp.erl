-module(benchmark_otp).
-export([start/0]).

start() ->
    Host = os:getenv("HOST", "127.0.0.1"),
    PortText = os:getenv("PORT", "8080"),
    case string:to_integer(PortText) of
        {Port, ""} when Port >= 1, Port =< 65535 ->
            start_listener(Host, Port);
        _ ->
            io:format(standard_error, "PORT must be an integer from 1 to 65535~n", []),
            erlang:halt(2)
    end.

start_listener(Host, Port) ->
    case inet:getaddr(Host, inet) of
        {ok, Address} ->
            {ok, _} = application:ensure_all_started(cowboy),
            Dispatch = cowboy_router:compile([
                {'_', [{"/[...]", benchmark_otp_handler, []}]}
            ]),
            {ok, _} = cowboy:start_clear(benchmark_http,
                [{ip, Address}, {port, Port}],
                #{env => #{dispatch => Dispatch}}),
            io:format(standard_error, "ready otp-cowboy http://~s:~p~n", [Host, Port]),
            receive stop -> ok end,
            cowboy:stop_listener(benchmark_http);
        {error, Reason} ->
            io:format(standard_error, "Invalid HOST: ~p~n", [Reason]),
            erlang:halt(2)
    end.
