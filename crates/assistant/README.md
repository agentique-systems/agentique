# agq-assistant

The Assistant (ROADMAP §4.1, §4.10, part `Assistant` in
`models/agentique/Agentique.sysml`): the AI agent the Operator works with in
the Conversation. It depends on the System State, the language core and
Providers (`agq-providers`), which talks to model providers through rig. Its
hand-written Claude client still uses the network directly until W5.7.

- `tools`: what the Assistant can do. `read_model` (an outline of the whole
  model, or one element's full text), `find_elements` and `get_problems`
  read the System State; every result is cut at about 8,000 tokens with a
  note on narrowing the request (R-34); `apply_changes` turns a request into
  one System State `Change` (one undo step) that the Studio applies exactly
  like an Operator edit, so locks ask the Operator first; `ask_operator` puts
  a question to the Operator. Tool input is untrusted: the turn checks it
  against the tool's schema (`check_input`), then `prepare` resolves names
  and tries every operation on a copy of the model before the change is
  handed over (ROADMAP §4.2).
- `turn`: the tool-use loop for one turn of the Conversation.
- `choice`: which model the Assistant uses (below).
- `provider_model`: the model through `agq-providers` (DeepSeek, OpenAI,
  OpenRouter; Anthropic through rig too, though the Studio still uses
  `claude` for Anthropic until W5.7). A provider's reasoning is stored as a
  `reasoning` block naming its provider and model, and sent back only to it.
- `claude`: the hand-written Claude API client (retired in W5.7).
- `skills`: the system prompt, compiled in from `skills/*.md`.
- `conversation`: the per-project Conversation. The model's content blocks
  are stored as returned, so a conversation continues after a restart.
- `model`: the language model interface. `ScriptedModel` is a deterministic
  stand-in for tests.

## How a turn works

The Operator's message is added to the conversation, then `turn::run` (or
`BackgroundTurn::start`, below) answers it:

1. Send the skills, the tool definitions and the conversation to the model;
   stream the reply (text, thinking as it may be shown, tool calls starting,
   tool input, the provider's id for a call, and the tokens used).
2. Add the reply to the conversation. If it asks for tools, check each call's
   input against the tool's schema and hand valid calls to the executor, in
   order. Invalid input (unreadable JSON, unknown fields, wrong values) is
   answered with an error result and never run.
3. Add all results as one entry and go back to 1, until the model has
   finished. A turn stops after 40 model calls, when the Operator stops it,
   on a refusal, when a reply is cut off at the output limit (its tool calls
   are not run), when the conversation is too long for the model, or on an
   error; a `Notice` entry says which, in plain words. The conversation is
   always left ready for the next message, and every tool call shown as
   started gets a `ToolFinished`, run or not.

The executor is the Studio's, because the Studio owns the project. For each
call it does what the Operator's own edits do, and returns a `ToolResult`
(the turn fills in its id):

```rust
match tools::prepare(project.state(), &call.name, &call.input) {
    Prepared::Answer(text) => ToolResult::answer(text),
    Prepared::Invalid(message) => ToolResult::error(message),
    Prepared::Question { question, options } => ToolResult::answer(/* the Operator's answer */),
    Prepared::Change(mut change) => match project.apply(change.clone()) {
        Ok(event) => ToolResult::applied(project.state(), &event),
        // Ask the Operator to confirm this change to the locked elements; if
        // they agree, set `change.confirmed = elements` and apply it again.
        Err(ApplyError::Rejection(rejection)) => ToolResult::rejected(project.state(), &rejection),
        Err(error) => ToolResult::error(error.to_string()),
    },
}
```

The Assistant never changes the System State itself, and a refused lock
leaves everything as it was; the model reads why and goes on.

### In the Studio: `BackgroundTurn`

`BackgroundTurn::start(model, conversation)` runs a turn on a background
thread and reports `BackgroundEvent`s.
The UI thread calls `next_event()` every frame (it never blocks):

- `Turn(TurnEvent::Stream(..))`: streamed text, thinking, a tool call
  starting, tool input arriving, and `Usage` (tokens) when a reply is
  complete; for the live Conversation panel and a cost display.
- `Turn(TurnEvent::ToolFinished(result))`: one tool call's result, for its
  card.
- `Turn(TurnEvent::Entry(entry))`: an entry to add to the Studio's copy of
  the conversation (then save it).
- `ToolCall { call, reply }`: carry the call out on the UI thread (above)
  and send the `ToolResult` on `reply`; the turn waits for it until the
  Operator stops the turn. After a stop, close any question or
  confirmation still open for the call instead of carrying it out.
- `Finished`: the turn is over.

`stop()` sets a flag the turn checks before every model call and tool call;
a reply that is streaming in is abandoned within 50 ms, even while the
network is silent, and a tool call waiting for the UI thread is given up.
Changes made before the stop stay and can be undone. Dropping the
`BackgroundTurn` stops the turn. Use a new model (`ModelChoice::start`) for
each turn.

## Configuration

Until Settings exists (W5.8), `ModelChoice::from_env` picks the model:

| Variable | Default | Meaning |
|---|---|---|
| `ANTHROPIC_API_KEY`, `DEEPSEEK_API_KEY`, `OPENAI_API_KEY`, `OPENROUTER_API_KEY` | none | Provider keys. Without any, the Conversation says so; the Surface works by hand as always. |
| `AGENTIQUE_PROVIDER` | the first provider with a key: Anthropic, DeepSeek, OpenAI, OpenRouter | `anthropic`, `deepseek`, `openai` or `openrouter`. |
| `AGENTIQUE_MODEL` | the provider's default (`claude-opus-5`, `deepseek-flash`, ...) | The model id. An unknown or unavailable model is named in the error. |
| `AGENTIQUE_EFFORT` | the model's default (`high`) | An effort level the model offers (`deepseek-flash`: `low`, `high`, `max`; Claude: `low` to `max`); another value falls back to the default. |

With only `DEEPSEEK_API_KEY` set, the Assistant runs on `deepseek-flash` at
effort `high` (C-35).

Errors are explained for the Operator and never lose the Operator's message:
a refused key (401), no permission (403), an unknown model (404), a
conversation too large (413), rate limits (429, with when to try again),
server errors, unavailability and overload (500, 502, 503, 504, 529), no
network, a reply cut off. Rate
limits, server errors and connection failures are retried twice (after one
and two seconds, or the wait the API asks for if that is at most ten seconds)
before the error is shown.

## Costs

Every model call is billed by tokens; one turn often makes several calls
(each tool round is one).

- **Model.** `claude-opus-5` costs $5 per million input tokens and $25 per
  million output tokens. Thinking is on (`thinking: adaptive`) and billed as
  output; `max_tokens` (64,000) caps thinking and reply together per call.
- **Effort.** `high` is the API default. On this model `medium` and `low` are
  strong and much cheaper and faster; lower `AGENTIQUE_EFFORT` first if costs
  or waits matter.
- **Caching.** The system prompt and tools are identical in every request and
  cached (an explicit breakpoint on the system prompt), and the growing
  conversation is cached too (top-level `cache_control`). Cached input costs
  about a tenth of the normal price; the cache lives five minutes after its
  last use.
- **Fallbacks.** For `claude-opus-5` the requests carry `fallbacks:
  "default"` with the beta header `server-side-fallback-2026-07-01`: if the
  model's safety classifiers decline a request, the API reruns it on its
  recommended fallback model (billed at that model's rates) instead of
  returning a refusal. Other models are sent without it. A refusal that
  remains ends the turn with a notice; nothing of the declined reply is run.
- **Limits.** A turn pauses after 40 model calls and waits for the Operator.
- **Showing costs.** Each complete reply reports its tokens as
  `StreamEvent::Usage` (full-price input, cache writes, cache reads,
  output); the Studio can add them up per turn.

## Skills

`skills/*.md`, in this order, form the system prompt: `assistant` (who the
Assistant is and how it works with the Operator), `modelling` (the SysML
subset, docs/subset.md), `simplicity` (the slop rules of §1.3 applied to
architecture), `ideas` (map ideas onto the architecture first), `decisions`
(ask on major decisions), `locks` and `tools` (using the tools well). Edit
them as Markdown; they are compiled in, so a change needs a rebuild.

## Tests

`cargo test -p agq-assistant` runs without the network: the tool contracts
and schema checks (`tests/tools.rs`); the loop with a scripted model and an
executor on a real System State, including lock refusal, invalid input,
questions, stop and the background `Assistant` (`tests/turn.rs`); and the
client, reading canned event streams and talking to a local server that
answers like the API (`tests/claude.rs`).

`cargo run -p agq-assistant --example smoke` runs one real turn against the
configured model, and does nothing without a key. Live runs pass a spend guard
(developer tooling in `examples/support/spend.rs`, not a product feature):
set `AGENTIQUE_SPEND_LOG` to a JSON-lines file outside the repository and
`AGENTIQUE_SPEND_STOP_USD` to a hard stop; every call is logged, and a run whose
worst case would pass the stop refuses to start.
