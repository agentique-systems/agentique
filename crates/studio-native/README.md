# agq-studio-native

The Studio (ROADMAP §4.6, part `Studio` in `models/agentique/Agentique.sysml`):
the Surface, the Panels (Inspector, Requirements, History) and the
Conversation with the Assistant, on egui and wgpu. Every model change is a
System State change applied through one path (`StudioApp::apply_change` in
`edit.rs`), whoever makes it; the project saves it.

## Conversation

The column on the right (`conversation.rs` for the state and tool calls,
`conversation_ui.rs` for drawing, `markdown.rs` for rendering messages).

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
- **Selecting text** (`markdown.rs`): dragging over the messages selects
  across paragraphs, code blocks and messages; past the top or bottom of the
  list it scrolls. The selection is kept by place in the text (message,
  block, character), not by widget, so it survives scrolling, messages
  outside the view (which are not laid out) and replies streaming in below.
  Ctrl+C copies it in order; a click clears it.

## Settings

Ctrl+, (or the Settings button) shows Settings in place of the Surface and
the Panels (`settings_ui.rs`; the table and `settings.json` are in
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

The `e-settings` journey (Scenario E without keys or the network) opens
Settings with Ctrl+, shows each section, searches with a synonym and closes:

```text
target\debug\agq-studio-native.exe --no-restore --session %TEMP%\agq-e\session.json --scenario e-settings --project %TEMP%\agq-e\demo --gallery %TEMP%\agq-e\shots
```

## Type, motion and screen readers

- **Tokens** (`tokens.rs`, ROADMAP §3.2): every size, radius, duration and
  easing curve, and the colour scales generated in OKLCH from three inputs
  (base hue, accent hue, contrast), with the component list and the states
  every component shows. `--fixture components` shows them all
  (`gallery.rs`); `theme.rs` moves onto the generated colours in W5.2.

- **Type**: Inter's variable font (`assets/fonts/InterVariable.ttf`, Inter
  4.1, OFL) at `wght` 400, 500 and 600 (`theme.rs`), not synthetic bold.
  `--ui-scale 1.5` scales the UI as display scaling does, for checking text
  at 100%, 150% and 200% on one display.
- **Motion** (`motion.rs`): a tween (a duration token along a Fluent 2
  easing curve) and a critically damped spring. Fit view is a 300 ms tween;
  following the selection uses springs, which keep their velocity when the
  target changes. Reduced motion makes both instant.
- **Screen readers** (`accessibility.rs`, AccessKit): every message is an
  article named by who wrote it and its text, every tool card a group named
  by its title and status; the Surface is a list whose items are the
  selected elements and the card under the pointer, which the GPU draws
  without widgets.

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
