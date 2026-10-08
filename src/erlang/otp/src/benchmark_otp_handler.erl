-module(benchmark_otp_handler).
-export([init/2]).

-define(MAX_BODY, 1048576).

init(Req, _Options) ->
    Method = cowboy_req:method(Req),
    Path = cowboy_req:path(Req),
    case {Method, Path} of
        {<<"GET">>, <<"/health">>} ->
            respond(Req, 200, <<"application/json">>, <<"{\"status\":\"ok\"}">>);
        {<<"GET">>, <<"/plaintext">>} ->
            respond(Req, 200, <<"text/plain; charset=utf-8">>, <<"hello, world!">>);
        {<<"GET">>, <<"/json">>} ->
            respond(Req, 200, <<"application/json">>, <<"{\"message\":\"hello, world!\"}">>);
        {<<"POST">>, <<"/echo">>} ->
            case read_all(Req, [], 0) of
                {ok, Body, Req1} ->
                    respond(Req1, 200, <<"application/octet-stream">>, Body);
                {too_large, Req1} ->
                    respond(Req1, 413, <<"text/plain; charset=utf-8">>, <<"Payload Too Large">>);
                {error, Req1} ->
                    respond(Req1, 400, <<"text/plain; charset=utf-8">>, <<"Bad Request">>)
            end;
        _ ->
            respond(Req, 404, <<"text/plain; charset=utf-8">>, <<"Not Found">>)
    end.

respond(Req, Status, ContentType, Body) ->
    Headers = #{
        <<"content-type">> => ContentType,
        <<"content-length">> => integer_to_binary(byte_size(Body))
    },
    {ok, cowboy_req:reply(Status, Headers, Body, Req), undefined}.

read_all(Req, Chunks, Total) ->
    try cowboy_req:read_body(Req, #{length => 65536, period => 30000}) of
        {ok, Chunk, Req1} ->
            maybe_complete(Req1, [Chunk | Chunks], Total + byte_size(Chunk), true);
        {more, Chunk, Req1} ->
            maybe_complete(Req1, [Chunk | Chunks], Total + byte_size(Chunk), false)
    catch
        _:_ -> {error, Req}
    end.

maybe_complete(Req, _Chunks, Total, _Done) when Total > ?MAX_BODY ->
    {too_large, Req};
maybe_complete(Req, Chunks, _Total, true) ->
    {ok, iolist_to_binary(lists:reverse(Chunks)), Req};
maybe_complete(Req, Chunks, Total, false) ->
    read_all(Req, Chunks, Total).
