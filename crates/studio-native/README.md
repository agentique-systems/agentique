# agq-studio-native

The Studio (ROADMAP §4.6, part `Studio` in `model/Agentique.sysml`):
the Surface, the Panels (Outline, Inspector, Requirements, History, Problems,
Objectives) and the Conversation with the Assistant and objectives'
threads, drawn with GPUI (C-48: the pinned snapshot `gpui-pre`, with the
unstyled `gpui-base` primitives for text fields and focus). Every model change is a System State change. The Operator's
edits, the Assistant's and the Library's go through one Studio path
(`Studio::apply_change` in `edit.rs`), so locks ask and each is one undo
step; undo, redo and opening go through `Project` directly; the project
saves each before it returns. Two writes into the model folder are not
changes, and say so: implementation links (`model/links.json`, outside undo,
C-50) and the URL shortener sample, whose text is written before the project
is first opened.

## How it is built

- **One state, many views.** `studio.rs` holds everything the Studio knows
  (the project, the scene and camera, the selection, the Conversation,
  Settings) without the toolkit; it is a GPUI entity. A change goes through
  `StudioExt::act` (`workspace.rs`), which says what it touched (`Dirty`:
  camera, selection, model, conversation, layout, appearance, status,
  overlay); each view listens only for what it draws. The docked columns are
  cached, so a Surface frame does not draw them again.
- **Views**: `workspace.rs` (title bar, docks and splitters, status bar,
  overlays, the frame ticker), `surface/` (input, gestures, overlays;
  `paint.rs` draws a frame with GPUI's quads, paths and shaped text in paint
  layers, only what is in view, by level of detail; `minimap.rs`),
  `panels/` (Outline, Library, Scenarios, Inspector and its fields with the
  Behaviour, Stand-in, Agent, Evidence and Implementation sections in
  `evidence.rs`, Run, Requirements, History, Problems, and Objectives: a
  dashboard of the objective the Conversation shows, its record, child
  objectives, the same commands and its latest steps), `conversation_view/`
  (the list, cards, Markdown, selection and objectives' threads),
  `objective_form.rs` (the objective's start form, shared by the
  Conversation and the Objectives panel),
  `palette.rs`, `dialogs.rs`, `welcome.rs`, `settings_view.rs`, `gallery.rs`.
- **Keys** (`commands.rs`): every command has its GPUI keystroke; shortcuts
  that type letters apply in the workspace except while a text field has the
  keyboard (`Workspace && !Input`), the Ctrl shortcuts everywhere.
- **The design system** (`ui/`): theme roles derived from the tokens, Lucide
  icons, buttons, fields, menus, dialogs, tooltips, chips, badges, key caps,
  switches and a segmented control, banners and empty states. Views take their
  controls from here; their own rows use the same theme roles.

## Scenarios, runs and code (C-50)

- `runs.rs`: the project's scenarios, runs on background threads (model,
  replay, walkthrough, code through the harness, live after the Operator
  confirms), results kept in the project's app data with their freshness
  (worked out again on every model change, undo included), the playback
  cursor and the marks the Surface draws (only for a current result), and
  writing scenarios through controls; the Assistant's run and result
  requests.
- `implementation.rs`: links, the per-project trusted-local choice, the
  executor, rounds of implementation checks, drift, opening linked code.
- `tasks.rs`: implementation tasks: a job and a git worktree each, the
  worker on its own thread, the Studio's own verification, the review with
  the patch, integration (only if the repository has not moved) or discard,
  continuing a task in its worktree, and interrupted tasks found on open.
- `live.rs`: how an agent is evaluated live or replayed (C-52). `prepare`
  admits one agent configuration and finds from the capability table how its
  model is called: a typed choice for a known pinned decision model (one
  required enum field with documented values, the library's confidence, one
  input item; the declared fields sent as data), chat with the answer
  template otherwise, or the Assistant's model, through chat, when the agent
  names none; a model no table knows is refused, never replaced. It names the
  binding that replay and live share. `plan` is what the Operator confirms:
  the model and how it is asked, at most how many calls and requests, what
  leaves the computer, a cost bound, what the confidence means and what the
  fallback does not do; it is frozen, a change before the start refuses it
  and a change during the run stops it. The clients stop at each call's
  deadline and on Stop; a typed decision's confidence is copied unchanged
  and grants nothing; provider failures end the evaluation, never a
  verdict. Known live cost joins the day's total; unknown cost is said.
  `screening_evaluation` (an ignored test) is the screening evaluation of
  the System One investigation's §7: the Operator's labelled cases through
  the typed, chat and blocklist arms, with consent, an allowance and the
  spend stop, observations written outside the repository for
  `tools/screening_report.py`.
- Journeys `i-scenarios` and `i-code` (`--features automation`).

## Conversation

The column on the right (`conversation.rs` for the state and tool calls,
`conversation_view/` for drawing: `mod.rs` the list and composer, `cards.rs`
the tool, question and thinking cards, `markdown.rs` the messages).

- **Sending.** Enter sends, Shift+Enter is a new line; Ctrl+L focuses the
  input, Ctrl+I (or "Insert selection") puts the selected elements' qualified
  names into it, Ctrl+J shows or hides the column. The input is cleared only
  once the message is in the conversation, so an error never loses it.
- **A turn** runs on a background thread (`agq_assistant::BackgroundTurn`);
  the Studio polls it every frame. Replies stream in as Markdown (paragraphs,
  headings, bold and italic in real weights, inline code, code blocks with a
  copy button, lists). An inline code span that names an element (qualified,
  or a unique simple name) is a link: clicking it selects the element on the
  Surface and shows it in the Inspector.
- **Tool calls** run on the UI thread: `tools::prepare`, then a change goes
  through `apply_change` like an Operator edit, so the Surface highlights
  it, it is one undo step, and a locked element opens the same lock
  confirmation (the Operator's answer is the tool result). Each call is a
  card: a spinner while it runs, then what it created or changed (as links),
  deletions and problems, expandable to the input and full result. Reading
  tools are shown smaller.
- **Questions** (`ask_operator`) are prompt cards with the options as
  buttons; a typed message answers an open question.
- **Stop** stops at once: a question or confirmation still open is closed,
  and every tool call of the turn not yet carried out is answered "not run".
  Changes made so far stay. "Undo the Assistant's changes" undoes the turn's
  changes (`Project::undo_since`), each redoable; when changes by others were
  made since the turn started (`SystemState::steps_since` tells), it says
  so: "Undo all changes since the Assistant started". **Retry** sends again
  after a failed or stopped turn; **Edit** replaces the last message and
  everything after it. Opening another project stops the turn and records
  its last results first.
- **Saved per project** in the Studio's local data, next to the session file
  (`projects/<folder>-<hash>/conversation.json`, conversation format 2; see
  `crates/assistant/README.md`), never in the project folder, after every
  entry. A conversation Stage 4 kept in `conversations/` is shown as a
  read-only transcript. "New conversation" starts again; the model is unaffected.
- **Model**: the provider, model and effort chosen in Settings, with the
  environment winning (a provider key such as `DEEPSEEK_API_KEY`, and
  `AGENTIQUE_PROVIDER`, `AGENTIQUE_MODEL`, `AGENTIQUE_EFFORT`; see the
  Assistant's README), named discreetly in the header. Without a key the
  column says how to add one and everything else works.
- **Cost** (R-42): beside the model, the running or last turn's estimated
  cost and today's total (UTC day, kept in `usage.json` beside the session
  file), from the dated list prices in `agq-providers`; an estimate, never a
  bill, and nothing is capped (C-37). Settings › Assistant › "Show estimated
  cost" hides it.
- **Thinking** shows as a collapsed row per step with its first line
  (R-31): Claude's summaries, or the reasoning of models that show it.
- **Objectives** (C-54, ROADMAP §3.6, §4.16: the Conversation is the one
  place for intent, agent communication, observation and steering). The
  composer addresses the Assistant or the objective shown ("To: the
  Assistant" / "To: the objective"; only the Operator's click, Ctrl+L or
  "Write to the objective" switches it). "Start as objective" beside Send
  (Ctrl+Enter in the message) shows the start form above the composer with
  the message as the intent: whether it explores, budgets (spend,
  improvements, attempts, hours, exploration steps), merge and adopt, and
  each role's model with any fallback, credential and who pays; nothing
  starts until Start (`objective_form.rs`, the same form the Objectives
  panel shows when it is not open here). The Assistant's
  `propose_objective` opens the same form; only the Operator starts it.
  While an objective runs or waits to continue, a bar above the list has
  its phase, spend and children with Write to it, Pause, Step, Resume,
  Stop and Continue: the palette's commands (`start-objective`,
  `message-objective`, `pause-objective`, `step-objective`,
  `resume-objective`, `stop-objective`, `continue-objective`), also the
  panel's buttons. The objective's thread (`conversation_view/thread.rs`,
  read from `objectives/<id>/thread.jsonl` from its tail, then followed
  as entries arrive) is in the list in time order with the conversation's
  entries, never sent to the Assistant's model: the Operator's messages
  ("you → the objective", with where each went: to the implementer at its
  next tool call, or waiting for the lead's next turn), directives (author
  → recipient with its model, a status chip from the record, a child's
  focus and budgets), results and Agentique's events, each agent's entry
  with its role and model; tool calls fold under their step ("12 tool
  calls"), each one's diff or command line a click further; a child
  objective's thread is nested under the directive that started it; a
  directive arriving while shown streams in at the observer speed
  (`control.speed`, within six seconds). A reply goes to the objective
  through the same path as the panel's message field
  (`Studio::message_objective`): a running objective records where it
  went; one not running keeps it in its thread for the lead's next turn.
  To interleave threads, the time each entry was added is kept beside the
  conversation (`conversation.times.json`); the conversation file
  (format 2) is unchanged.
- **Selecting text** (`conversation_view/markdown.rs`): dragging over the messages selects
  across paragraphs, code blocks and messages; past the top or bottom of the
  list it scrolls. The selection is kept by place in the text (message,
  block, character), not by widget, so it survives scrolling, messages
  outside the view (which are not laid out) and replies streaming in below.
  Ctrl+C copies it in order; a click clears it.

## Library

Building blocks (C-49, ROADMAP §4.13; the blocks and the copying are in
`agq-library`, the Studio's side is `library.rs`, `panels/library.rs`,
`panels/block_preview.rs` and `panels/reuse.rs`). A block is an ordinary
definition; using one adds a usage typed by it and copies the definitions it
needs into the project's `Library` package (collapsed on the Surface until
expanded), so the project stands alone.

- **Finding.** Ctrl+Shift+L shows the Library tab beside the Outline: type to
  search (names first, then qualified names and words from the docs; the
  matched letters are marked), filter by scope (Built-in, Project, Mine) and
  kind, and see the selected block's preview (its boundary ports, its parts
  in columns and their connections), doc, ports, values and usages. The
  arrows move through the results, Enter inserts.
- **Using.** Enter, a double-click or dragging onto the Surface or a container
  inserts a block (a drag onto a card's port connects it too); Shift+A opens
  "Insert from Library…" in the palette. With a port selected, "What can
  connect here?" lists only the blocks whose ports fit it, by the language's
  own rule, and inserting one connects it. A name the project already uses
  for something different asks: use the project's, or copy under another name.
- **Inside a block.** Enter or a double-click on a composite usage opens its
  definition on the Surface, with a breadcrumb and Back (Backspace,
  Alt+Left); inherited parts are dashed, overrides have an accent edge.
  Shift+F12 finds usages. The Inspector's Definition section says what
  changing the definition changes, and lists the values the element takes
  from its definition, each with Override (a value here only) or Reset.
  "Specialise…" makes a variant that leaves the original as it is; changing a
  definition with several usages asks first and offers to specialise instead.
- **Making blocks.** "Create building block from selection" turns selected
  parts into a definition and one usage of it, with ports where connections
  cross the boundary (one change, one undo). "Save to My Library…" keeps a
  definition and what it needs in `library\My Library.sysml` beside the
  session file; nothing is published anywhere.

## Settings

Ctrl+, (or the Settings button) shows Settings in place of the Surface and
the Panels (`settings_view.rs`; the table and `settings.json` are in
`settings.rs`). Sections: Providers (paste a key, Test, Save; the key goes to
the Windows Credential Manager and only its hint is shown again; the model
list with capabilities and list prices; for Anthropic also the Operator's
Claude subscription token from `claude setup-token`, which only the Claude
Agent runtime's sessions use, and which of the two they use when both are
there), Assistant (provider, model, effort; the Claude Agent runtime's card
says which credential each provider the agents use has, who pays, and
whether this computer has a Claude login and why it is not used), Agents
(C-54: each Orchestrator role's model, effort and fallback, with what it
resolves to now), Appearance, Keyboard and About; the search box finds rows by label,
description and synonyms. Choices apply at once and are saved to
`settings.json` beside the session file. The theme, contrast and reduced
motion commands change the same settings. Environment variables
(`DEEPSEEK_API_KEY`, `AGENTIQUE_PROVIDER`, ...) win over Settings; an
Anthropic key saved in Settings is not used until W5.7 moves Anthropic onto
the provider layer.

The `d-daily` journey (Scenario D) starts from the first run's welcome with
the URL shortener sample, fits (Shift+1), zooms to a selected card (Shift+2),
goes to an element (Ctrl+P), visits the three views, lists the shortcuts (?)
and records a checkpoint (Ctrl+S):

```text
target\debug\agq-studio-native.exe --no-restore --session %TEMP%\agq-d\session.json --scenario d-daily --project %TEMP%\agq-d\demo --gallery %TEMP%\agq-d\shots
```

The `e-settings` journey (Scenario E without keys or the network) opens
Settings with Ctrl+, shows each section, searches with a synonym and closes:

```text
target\debug\agq-studio-native.exe --no-restore --session %TEMP%\agq-e\session.json --scenario e-settings --project %TEMP%\agq-e\demo --gallery %TEMP%\agq-e\shots
```

The `h-library` journey (Scenario H) starts from the URL shortener sample,
searches the Library, previews and inserts a composite, connects a block by
"What can connect here?", drags one onto the Surface, opens a definition and
comes back, specialises and overrides a value (the original stays), finds
usages, creates a block from a selection, saves it to My Library, uses it in
a second project, has the scripted Assistant reuse a block and model a plain
definition when none fits, and undoes the Assistant's changes:

```text
target\debug\agq-studio-native.exe --no-restore --session %TEMP%\agq-h\session.json --scenario h-library --project %TEMP%\agq-h\demo --gallery %TEMP%\agq-h\shots
```

## Type, motion and screen readers

- **Tokens** (`tokens.rs`, ROADMAP §3.2): every size, radius, duration and
  easing curve, and the colour scales generated in OKLCH from three inputs
  (base hue, accent hue, contrast), with the component list and the states
  every component shows. `ui/theme.rs` turns them into roles. `--fixture
  components` shows every token and component, the Surface's own drawing
  and a real Conversation (`gallery.rs`).

- **Type**: Inter 4.1 (OFL) at 400, 500 and 600, as three static fonts
  (`assets/fonts/Inter-*.ttf`) instanced from Inter Variable at text optical
  size, because GPUI's Windows text system does not select a variable font's
  weights; JetBrains Mono NL for names and values.
  `--ui-scale 1.5` scales the UI as display scaling does, for checking text
  at 100%, 150% and 200% on one display.
- **Motion** (`motion.rs`, `ui/primitives.rs`): the camera's 300 ms tween,
  change highlights that hold and fade, and critically damped springs for
  switches, sliding selections and docks opening. Reduced motion (Windows'
  "Animation effects", or Settings) makes them instant.
- **Screen readers** (GPUI's AccessKit roles): every message is an article
  named by who wrote it and its text, every tool card a group named by its
  title and status; the Surface is a list whose items are the selected
  elements and the card under the pointer, which the Surface paints without
  elements of their own.

Tests (`cargo test -p agq-studio-native conversation`) drive the Conversation
with a scripted model on a real project; the `a-assistant` journey does the
same through the UI:

```text
cargo build -p agq-studio-native --features automation
target\debug\agq-studio-native.exe --no-restore --session %TEMP%\agq-a\session.json --scenario a-assistant --project %TEMP%\agq-a\demo --gallery %TEMP%\agq-a\shots
```

The Conversation benchmark (ROADMAP S4.1, gate G3) generates a
200-message conversation in a new project, scrolls it and streams a reply
into it at about 100 tokens a second, and reports frame times:

```text
cargo build --release -p agq-studio-native --features automation
target\release\agq-studio-native.exe --no-restore --session %TEMP%\agq-chat\session.json --scenario chat --project %TEMP%\agq-chat\project --scenario-report %TEMP%\agq-chat\report.json
```

## The control interface (C-53, C-54)

Agents operate the real, visible Studio without seeing pixels (ROADMAP
§4.16; `control/`): the Assistant through its `observe_app` and
`act_in_app` tools, the Orchestrator's explorers and evaluators through a
test instance's local endpoint (`--control <file>`: a port and a token on
127.0.0.1; `control/server.rs`).

- **Observation** (`op: observe`): the identity, the screen and its
  `screenRevision`, view, panels, dialog, palette, selection, status and
  problems, the conversation, tasks and builds; the project's folder,
  revision and, in a full observation (`detail: "full"`, or `digest:
  true`), `digest` (16 hex digits of the SHA-256 of the model's printed
  text, what History saves, worked out once per revision: undo restores it
  exactly); `agents` (the gate, actions held, the window's
  `holder`, the `speed`); every drawn control (`ui/target.rs`: id, role,
  label, value except a key's, enabled, selected, focused (fields, buttons
  and switches), region, the
  bounds of its visible part, `hidden`, and `operatorOnly` where agents may
  not act on it); and the commands, available or why not, likewise marked.
  `operatorOnly` comes from the same rules that refuse an agent's action up
  front (the Operator's commands and regions, and one table of controls by
  id), so the mark follows those refusals; the effects themselves are
  refused where they happen as well. A menu's items are controls of the
  region that opened it (`Menu::owner`).
- **Labels.** Every control an agent can operate (button, field, tab,
  option, item, switch, link) has a label a person reads, not just its
  machine id: the palette's rows (`palette-<command>`, `palette-element-<id>`)
  and search field (`palette-search`), switches (role `switch`, value
  `on`/`off`), the Inspector's and Settings' fields, the title bar's search
  and window buttons, menu items. `tests/control_journey.rs` asserts the
  rule on the welcome screen, a dialog, the Surface, the palette, Settings
  and the Objectives panel.
- **Actions** (`op: act`, with `agent`, `why`, an optional `goal`,
  `expect.instance` and `observed`): commands, click, fill, key, type,
  scroll, select, open a project, wait (`until`: `dialog`, `screen`,
  `control` with `enabled`, `statusContains`, `idle` for every job, or
  `conversationIdle` for the Conversation's turn alone). Stale ones
  (another instance, a screen that changed, a control gone or disabled) and
  the Operator's own are refused before anything happens, and the
  Operator's own effects are refused where they happen too. A refused
  action's answer carries `kind` beside `error`, so clients need not read
  the words: `operator-own`, `stale`, `gone`, `disabled`, `unavailable`,
  `held`, `stopped`, `expired`, `invalid`, `timeout` or `failed`
  (`control::Refusal`).
- **A test instance** (`--test-instance`, for a Studio started with app data
  of its own: it refuses to start without `--session`, or with the
  Operator's; the Orchestrator is to pass it in W12.5): agents may also use the
  Conversation as a person would (focus the composer, type, send or answer
  with Enter or Send, stop the turn, open cards, scroll) and undo and redo,
  so exploration tests them through the real input handlers. Steering the
  turn, retrying, editing, a new conversation, the model, Settings,
  locking, appearance, approvals and the agents chip stay the Operator's,
  and the `operatorOnly` marks follow. Keys come only from the environment
  there (`agq_providers::keys::without_store`): the instance never spends a
  key of the Operator's that it was not given, and Settings › Providers
  says so. The observation's `conversation` says whether a turn is
  `running`, the current or last turn's `toolCalls` (the last 12,
  `toolCallsOmitted` counting earlier ones: tool and state, running, done,
  failed with its `error`, or not run), its `notices` since the last
  message, the `error` a failed turn ended with, `keyMissing`, and `usd`,
  the conversation's estimated spend since the Studio started (null when
  some of it is unpriced), all bounded. Its messages, tool cards and
  thinking rows are controls (items), the composer is the field `Message`,
  Send and Stop are `send` and `stop`. The Conversation's text (the
  `lastMessage`, the `lastReply`, the labels of its messages, tool cards,
  questions and thinking, any value such as the Operator's unsent draft, a
  message the Operator added to a running turn) is observed only in a test
  instance: in the Operator's own window an observation could reach an
  agent's provider, so its items read "Message 2 from you", "Reply 3, part
  2", "Tool call 4" or "Tool error 4", "A question for you", numbered by
  their own count, with their ids unchanged. An objective's thread is in
  the Conversation too (C-54): its rows are items (`thread-<objective>-<n>`,
  folds `thread-fold-<objective>-<n>`, a click opens one), the composer's
  addressee is `conversation-to`, a step's Reply `thread-reply-…`; in a
  test instance agents read, expand and reply in it as a person would,
  while starting, steering and stopping objectives (`objective-…`
  controls and commands) stay the Operator's everywhere; in the
  Operator's window its rows read "Directive, entry 3, by lead" without
  their text. The observation's `objective` gives the objective shown: its
  state, phase, spend, children, whether the start form is open, and its
  thread's size and last entry (kind, author, where it went; its text and
  the intent only in a test instance); `conversation.addressed` says whom
  the composer addresses. `--assistant-stand-in` (a test instance only)
  runs its Assistant on a scripted stand-in: no network, no key, no cost;
  it reads the model and answers in a line, so a turn can be started and
  observed where no credential may be given. A runtime given for the
  process like this (or a journey's scripted Assistant,
  `ConversationPanel::use_given`) is never replaced when Settings change
  or the credentials are read again, so a scripted run never reaches a
  real model whatever keys the computer holds.
- **One agent at a time.** The first agent to act holds the window until it
  sends `op: release` or has been idle for 30 s (`control::IDLE`); another
  agent's action is refused with "the window is in use by <agent>; act in
  your own test instance, or wait". Observing and waiting are never refused,
  nor is the Operator's own input. The Orchestrator's own steps by rule
  through the endpoint (agent `orchestrator`, such as cancelling a dialog in
  its way) are the supervisor's and pass a hold. In a test instance, the
  Assistant's turn that an agent's message started acts within that agent's
  hold (and keeps it); when its turn ends, the Assistant holds nothing.
- **Observer mode.** `control.speed` (Settings › Appearance, or
  `--control-speed` for a process; default `observe`): `observe` types about
  12 characters a second and rings the target with the agent's label for
  300 ms before a click; `fast` types a character a frame; `instant` acts at
  once, as tests want. At `observe` and `fast`, clicks ripple and scrolls
  show which way. The agents chip in the title bar shows the action in
  progress or the latest outcome for a few seconds (the agent, its goal,
  its decision, done or refused with the reason) and offers Pause, Step and
  Stop; Pause and Step take effect between typed characters, Stop ends the
  action at once and refuses agents until Resume (a press still down is
  let go away from its control, as the agent's input, so it becomes
  neither a click nor a drag); stopping the Assistant
  ends its action in progress the same way. The endpoint's holder has the
  same through `op: gate` (`pause`, `step`, `run`, `stop`), except that it
  cannot lift a Stop the Operator gave in the window. Nothing is
  drawn and no frame is asked for while no agent acts.
- **Trace** (`op: events`): every action and refusal, with the agent, its
  `why` and `goal`, and who held the window.

```text
cargo test -p agq-studio-native --test control_journey -- --ignored --nocapture
```

## Journeys, screenshots and benchmarks

With `--features automation` the Studio can drive itself: a journey runs one
step at the start of every frame, through the window's own input dispatch
(pointer, keys, typed text), and checks the Studio's state
(`automation.rs`); the camera and Conversation benchmarks do the same
(`stress_automation.rs`). Controls record where they are drawn in every
build (`ui/target.rs`), for the journeys and the control interface alike.
`--screenshot` and `--gallery` save frames with GPUI's frame capture, which
needs the same feature.

```text
cargo build --release -p agq-studio-native --features automation
target\release\agq-studio-native.exe --no-restore --session %TEMP%\agq-ref.json --frames 2 --metrics <out>\start.json
target\release\agq-studio-native.exe --no-restore --session %TEMP%\agq-ref.json --fixture stress1000 --scenario stress --scenario-report <out>\stress-1k.json
target\release\agq-studio-native.exe --no-restore --session %TEMP%\agq-ref.json --fixture stress10000 --scenario stress --scenario-report <out>\stress-10k.json
```

A journey's settings file sits beside its `--session` file, so a journey
never touches the Operator's own; a `settings.json` there with
`{ "format": 1, "appearance.theme": "dark" }` runs it in the dark theme.
