# agq-studio-native

The Studio (REALIGNMENT §3.6, part `Studio` in `models/agentique/Agentique.sysml`):
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
  (`conversations/<folder>-<hash>.json`), never in the project folder, after
  every entry. "New conversation" starts again; the model is unaffected.
- **Model**: `ClaudeModel::from_env()` (`ANTHROPIC_API_KEY`,
  `AGENTIQUE_MODEL`, `AGENTIQUE_EFFORT`), named discreetly in the header.
  Without a key the column says so and everything else works.

Tests (`cargo test -p agq-studio-native conversation`) drive the Conversation
with a scripted model on a real project; the `a-assistant` journey does the
same through the UI:

```text
cargo build -p agq-studio-native --features automation
target\debug\agq-studio-native.exe --no-restore --session %TEMP%\agq-a\session.json --scenario a-assistant --project %TEMP%\agq-a\demo --gallery %TEMP%\agq-a\shots
```
