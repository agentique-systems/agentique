# agq-providers

Providers (ROADMAP §4.7, §4.8; part `Providers` in
`model/Agentique.sysml`): talks to model providers through rig,
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
- **Jev** (`Providers::decide`, module `jev`): TypeSafe AI's typed decisions
  for fast agents (C-35), through a thin client until the migration to the
  released `rig-typesafeai` 0.43 (C-34, C-52 step U): yes or no (the
  probability of yes, with no confidence), a choice among 2–255 options or a
  score on 2–10 levels about a state (each with probabilities and a
  confidence, the model's claim, not a measured reliability); pinned to
  `jev-1.13.0`; never the Assistant's model (it has no tools).
  - *Before sending*: a model, a text or object state, instructions for
    every question, option and level counts and non-empty names, and a
    local request limit of 256 KiB (bytes, not a token count: the API's
    token limits are not checked here).
  - *The reply* is read whole, up to a local 1 MiB limit (larger is
    refused, never cut), then checked against the request: the pinned model
    itself (for an alias, a fixed version), every question id answered once
    with the kind asked, exactly the options or the levels and their legend,
    probabilities in [0, 1] summing to one within `rig-typesafeai`'s
    tolerance (1e-3, or min(n × 0.005, 0.02) when all are whole hundredths),
    the selection at a maximum within 1e-6, a score in range and within its
    distribution's mean, and no key named twice. Raw values are kept as
    sent. A reply that fails is `ErrorKind::InvalidReply`, never an answer.
  - *Usage* says when a count is missing (`DecisionUsage`): unknown is never
    zero, and only complete usage has a cost. A failure (`DecisionFailure`)
    says how many requests were sent, since a sent request may be billed,
    and keeps the usage a reply reported.
  - *Errors*: 401, 402/403, 404, 400/413/422 and 5xx map to plain kinds; an
    error reply is shown as a 300-character single-line excerpt without the
    key. 429 and 529 are retried twice, after `retry-after-ms` or
    `retry-after` seconds (an HTTP date is not read) or 1 s then 2 s; a wait
    over 10 s is not waited for.
  - *Deadlines and cancellation* (C-52): `decide_start(request, deadline)`
    returns a `DecisionHandle`, like `chat`'s handle: one monotonic
    deadline covers every request, the reading of each reply, the retries
    and their waits (a retry is made only when its wait fits; otherwise the
    failure says no time was left), and the result is `TimedOut` when it
    passes. `cancel` and dropping the handle stop it at once: nothing more
    is sent, a wait ends, the connection closes, and a reply or error that
    arrives afterwards is never delivered (whether the provider stops
    working or billing is not known). `decide` waits for one decision with
    a 30-second deadline. Decisions share one HTTP client, so connections
    are reused. `cargo run -p agq-providers --example jev` makes one live
    decision under a spend log and stop.
- **Capabilities** (`capabilities(&model)`) and **prices** (`price(&model)`, a
  dated table, an estimate) are data. Code outside this crate asks them, never a
  provider's name (§8.7).
- **Keys** (module `keys`, R-25, C-36): a non-empty environment variable
  (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `OPENROUTER_API_KEY`,
  `DEEPSEEK_API_KEY`, `TYPESAFE_API_KEY`) wins; otherwise the key stored in the
  Windows Credential Manager (a generic credential `agentique:<provider>`,
  persistence Local, at most 2,560 bytes), written with `keys::store` and
  removed with `keys::remove`; or an explicit `with_key` (tests, and testing a
  key before saving it). One thread owns all credential access, with a
  five-second timeout per call. Without a credential store there is no
  plain-text fallback: only the environment. `keys::hint` gives what Settings
  shows instead of a key (`sk-ant-…a1B2`). A key leaves this crate only in the
  request to its own provider; an endpoint override never gets the
  environment's or the stored key.
- **Key test and models** (for Settings, E2): `check_key` calls an endpoint
  that needs the key and runs no model (Anthropic and OpenAI `GET
  /v1/models`, OpenRouter `GET /api/v1/key` (not verified live), DeepSeek `GET
  /user/balance`, TypeSafe AI `GET /v1/models`) and answers works, refused,
  no access, rate limited, cannot reach or missing; `list_models` returns the
  provider's models with context window, the effort levels the provider
  lists, the capability table's entry and the dated price. `cargo run -p
  agq-providers --example keys` shows both for the configured keys (free).
- **Errors** are written for the Operator: a missing or refused key names its
  variable; no access, unknown model, rate limits, unavailability and lost
  connections each say what to do. Rate limits, server errors and lost
  connections are retried twice, only before anything streamed: after the wait
  the provider asks for if that is at most ten seconds, otherwise after one,
  then two seconds (ten seconds for a rate limit without a wait). A stream
  that ends without the provider's final record is a lost connection, never a
  reply; a tool call whose input is not JSON stays in the reply with its raw
  text, so the turn answers it with an error.
- **Server-side refusal fallbacks** (C-27) on `claude-opus-5`, through a thin
  adapter (Q-18, `src/fallback.rs`) until rig reads Anthropic's `fallback`
  content block: rig's Anthropic client gets an HTTP client that removes the
  block from the stream before rig parses it. After a switch, the declined
  model's reasoning and tool calls are dropped from the reply; its text stays.
- **Not yet through rig**: explicit cache breakpoints, marked in the
  capability table; the Studio keeps the hand-written Claude client for
  Anthropic until W5.7.
- **Async stays inside**: one background tokio runtime; the API is synchronous.
  Cancelling drops the request or stream, which closes the connection.
- **TLS**: rig and reqwest use `native-tls` (schannel on Windows), so no C
  crypto library is built on the reference machine (S4.2, P4). Linux builds use
  OpenSSL.

## Tests

`cargo test -p agq-providers` runs without the network: a local server answers
like each provider with canned streams and records the requests (reasoning and
effort parameters, the reasoning sent back, tool results), plus a refused key,
a retried rate limit, a missing key, a stop within 200 ms of a silent stream
and a reply continued by a fallback model; the fallback filter is also tested
on streams split into chunks of 1 to 97 bytes. Only DeepSeek has been tried
live (§7.6, C-35); the Anthropic, OpenAI and OpenRouter paths are tested on
canned streams only. The fallback filter assumes, untried live, that the
declined model's last block ends before the `fallback` block starts, as
Anthropic's documentation shows.
