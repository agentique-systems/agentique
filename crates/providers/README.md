# agq-providers

Providers (ROADMAP §4.7, §4.8; part `Providers` in
`models/agentique/Agentique.sysml`): talks to model providers through rig,
reads keys, knows each model's capabilities and reports usage. It is the only
crate that depends on rig, tokio or reqwest (R-21, R-41); none of their types
appear in its API.

```rust
let providers = Providers::new(); // keys from the environment
let mut call = providers.chat(ChatRequest {
    model: ModelRef::new(Provider::DeepSeek, "deepseek-flash"),
    effort: Some("high".into()),
    max_output_tokens: 32_000,
    system, tools, messages,
});
while let Some(event) = call.next_event(Duration::from_millis(20)) { /* ... */ }
call.cancel(); // stops at once; the call ends with Finished(Err(Cancelled))
```

- **Providers** (C-35): Anthropic, OpenAI (Responses API), OpenRouter and
  DeepSeek, all through rig 0.42.0, pinned exactly. One generic function streams
  every provider; what differs (the client, reasoning and effort parameters) is
  data. rig's agent loop is not used: the Assistant's turn loop keeps
  Agentique's policy.
- **Events** (the Assistant's event protocol): text, thinking (reasoning or
  summaries), a tool call starting under a stream id, its input as it arrives,
  the provider's id for the call once known, usage, and `Finished` with the
  reply or a plain error. Every call ends with exactly one `Finished`.
- **Messages** are provider-neutral (R-23): text, reasoning (kept to be sent
  back unchanged; DeepSeek refuses a history without it), tool calls and tool
  results.
- **Capabilities** (`capabilities(&model)`) and **prices** (`price(&model)`, a
  dated table, an estimate) are data. Code outside this crate asks them, never a
  provider's name (§8.7).
- **Keys** come from `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `OPENROUTER_API_KEY`
  or `DEEPSEEK_API_KEY` (the Credential Manager arrives with Settings, W5.8), or
  from `with_key` (tests, and testing a key before saving it). A key leaves this
  crate only in the request to its own provider.
- **Errors** are written for the Operator: a missing or refused key names its
  variable; no access, unknown model, rate limits, unavailability and lost
  connections each say what to do. Rate limits, server errors and lost
  connections are retried twice (after one, then two seconds, or the wait the
  provider asks for if that is at most ten seconds), only before anything
  streamed.
- **Async stays inside**: one background tokio runtime; the API is synchronous.
  Cancelling drops the request or stream, which closes the connection.
- **TLS**: rig and reqwest use `native-tls` (schannel on Windows), so no C
  crypto library is built on the reference machine (S4.2, P4). Linux builds use
  OpenSSL.

## Tests

`cargo test -p agq-providers` runs without the network: a local server answers
like each provider with canned streams and records the requests (reasoning and
effort parameters, the reasoning sent back, tool results), plus a refused key,
a retried rate limit, a missing key and a stop within 200 ms of a silent
stream. Only DeepSeek has been tried live (§7.6, C-35); the Anthropic, OpenAI
and OpenRouter paths are tested on canned streams only.
