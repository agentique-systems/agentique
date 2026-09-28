# agq-studio-native

The Studio (ROADMAP §4.6, part `Studio` in `models/agentique/Agentique.sysml`):
the Surface, the Panels (Outline, Inspector, Requirements, History, Problems)
and the Conversation with the Assistant, drawn with GPUI (C-48: the pinned
snapshot `gpui-pre`, with the unstyled `gpui-base` primitives for text fields
and focus). Every model change is a System State change applied through one
path (`Studio::apply_change` in `edit.rs`), whoever makes it; the project
saves it.

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
  `panels/` (Outline, Inspector and its fields, Requirements, History,
  Problems), `conversation_view/` (the list, cards, Markdown and selection),
  `palette.rs`, `dialogs.rs`, `welcome.rs`, `settings_view.rs`, `gallery.rs`.
- **Keys** (`commands.rs`): every command has its GPUI keystroke; shortcuts
  that type letters apply in the workspace except while a text field has the
  keyboard (`Workspace && !Input`), the Ctrl shortcuts everywhere.
- **The design system** (`ui/`): theme roles derived from the tokens, Lucide
  icons, buttons, fields, menus, dialogs, tooltips, chips, badges, key caps,
  switches and a segmented control, banners and empty states. Views take their
  controls from here; their own rows use the same theme roles.

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
- **Selecting text** (`conversation_view/markdown.rs`): dragging over the messages selects
  across paragraphs, code blocks and messages; past the top or bottom of the
  list it scrolls. The selection is kept by place in the text (message,
  block, character), not by widget, so it survives scrolling, messages
  outside the view (which are not laid out) and replies streaming in below.
  Ctrl+C copies it in order; a click clears it.

## Settings

Ctrl+, (or the Settings button) shows Settings in place of the Surface and
the Panels (`settings_view.rs`; the table and `settings.json` are in
`settings.rs`). Sections: Providers (paste a key, Test, Save; the key goes to
the Windows Credential Manager and only its hint is shown again; the model
list with capabilities and list prices), Assistant (provider, model, effort),
Appearance, Keyboard and About; the search box finds rows by label,
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

## Journeys, screenshots and benchmarks

With `--features automation` the Studio can drive itself: a journey runs one
step at the start of every frame, through the window's own input dispatch
(pointer, keys, typed text), and checks the Studio's state
(`automation.rs`); the camera and Conversation benchmarks do the same
(`stress_automation.rs`). Controls record where they are drawn only in this
build (`ui/target.rs`). `--screenshot` and `--gallery` save frames with
GPUI's frame capture, which needs the same feature.

```text
cargo build --release -p agq-studio-native --features automation
target\release\agq-studio-native.exe --no-restore --session %TEMP%\agq-ref.json --frames 2 --metrics <out>\start.json
target\release\agq-studio-native.exe --no-restore --session %TEMP%\agq-ref.json --fixture stress1000 --scenario stress --scenario-report <out>\stress-1k.json
target\release\agq-studio-native.exe --no-restore --session %TEMP%\agq-ref.json --fixture stress10000 --scenario stress --scenario-report <out>\stress-10k.json
```

A journey's settings file sits beside its `--session` file, so a journey
never touches the Operator's own; a `settings.json` there with
`{ "format": 1, "appearance.theme": "dark" }` runs it in the dark theme.
