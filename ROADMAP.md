# ROADMAP

**Status:** The governing direction for Agentique and the single governing
text (C-47). It carries forward every still-valid decision of `REALIGNMENT.md`,
which W4.1 retired to git history (its last version is on `main` at
`6fc90b78`). Where any other document, register or agent instruction conflicts
with this one, this one wins until that file is reconciled (§8).

**Date:** 2026-09-27
**Baseline inspected:** `main` at `6fc90b78` (Stages 0–3 built, pending the
Operator's acceptance)
**Origin:** a discovery interview with the Operator (six rounds, 2026-09-27); a
read-only investigation of the repository; measurements of the running Studio
on the Operator's machine; and research into rig, agent loops, settings
interfaces, visual design, UI toolkits, AI-driven components and fast model
modes. Research notes, screenshots and measurements are kept outside the
repository (§8.3). Every claim about an external product carries a numbered
source (§10).

**Adoption.** Merging this document was the Operator's adoption of it. The
first work item of Stage 4 (W4.1) removed `REALIGNMENT.md` and pointed
`AGENTS.md`, `README.md`, `docs/stages.md` and the crate READMEs here, so that
no competing version remains (§8.2).

**How to read the labels**

| Label | Meaning |
|---|---|
| **C-n** | Confirmed decision by the Operator |
| **R-n** | Recommendation (engineering judgment, open to revision) |
| **A-n** | Assumption not yet verified |
| **Q-n** | Open question |
| **D-n** | Decision carried forward from the records retired in Stage 0 |
| *(provisional)* | The recommendation depends on a spike or experiment |

IDs continue `REALIGNMENT.md`'s numbering and are never reused. Section 7 lists
all of them. Evidence is marked **measured** (by this investigation, release
build, on the Operator's machine), **derived** (computed from measured parts),
**not tried**, or **not verified** (no primary source found).

---

## 1. Vision and purpose

### 1.1 What Agentique is

Agentique is a **native desktop application in which a person and AI agents
design, simulate and implement systems together, working at the level of
system architecture rather than code.**

It is where a project's **development** lives for the project's whole
lifecycle (C-1). The project's artifacts (source code, repositories, running
services) live wherever they normally would. Agentique links to them and checks
them; it does not host them (C-1, C-7).

The Operator works mainly through the **Studio**, which has two equal ways to
work (C-3):

- **The Surface.** A visual, spatial window into the system being built. The
  Operator explores, inspects, analyses, adjusts, fixes and improves the system
  directly here, with side panels for detail.
- **The Conversation.** A chat with an AI **Assistant**. The Assistant has
  tools and skills to understand and change the system. It will later become an
  **Orchestrator** that directs other assistants (C-5).

Both work on one shared **System State**: the single, live, authoritative
description of the system (C-2). A change from either side appears on the
Surface immediately.

The System State is built from **KerML and SysML** concepts: parts, ports,
interfaces, connections, items, attributes, requirements, states, actions and
so on (C-4). These are the "Lego pieces". Their definitions, and the rules for
which pieces fit together, come from decades of shared systems-engineering
research instead of a vocabulary an AI invented.

### 1.2 Who it is for

Agentique is built for **the Operator: one person, on their own Windows
machine** (C-9). It is not driven by a business or marketing objective. The
first systems it builds are **software systems** (C-1).

### 1.3 The central promise: a cure for AI slop

Agentique exists because AI agents writing software directly tend to produce
**slop**. The Operator defines slop as follows (C-10):

1. **Unnecessary complexity.** Over-engineering instead of the simple solution.
2. **Shallow concepts.** Ideas are not thought through or generalised, so a
   project fills up with many independent or loosely related subsystems that
   should have been one well-designed concept.
3. **Invented naming.** Creative names instead of neutral, standard technical
   terms, which makes the system hard to follow.
4. **Literal compliance.** Following the words of an instruction rather than
   its intent. Agents must follow instructions *and* understand what the person
   actually means.

Two further failure modes are central to the promise:

5. **Drift.** An imperfectly explained new idea causes an agent to refactor an
   established, working part and break other systems. Repeated, this becomes an
   endless loop (C-11).
6. **Hidden architecture.** The system's structure is buried in code. Agents
   rarely draw it unless asked, so nobody sees the whole (C-12).

The **non-slop** target is solutions that are simple, clean, modular, scalable,
understandable, maintainable and stable, with **a stable core and
experimentation at the edges**, never experimentation at the core (C-10).

Agentique delivers this through three mechanisms:

- **Architecture-first thinking.** Most slop starts in architecture, not in
  low-level code. Making the architecture explicit, primary and something you
  work in directly is the main cure (C-12).
- **Visible, first-class architecture.** You work with the system as one works
  with a world in a game engine such as Unreal Engine, not through occasional
  diagrams (C-12).
- **Protection of established decisions.** Parts can be **locked**. Changing a
  locked part requires the Operator's explicit confirmation (C-11).

### 1.4 Dogfooding

This repository was produced largely by AI agents working without a tool like
Agentique, and before the realignment it showed every failure mode above (the
retired state is preserved at the tag `archive/pre-realignment`). Agentique must
eventually be able to rescue repositories like it, and Agentique must itself be
designed by the principles it promises (C-13, C-20). From this phase on,
Agentique's own Assistant is modelled in Agentique's architecture as its first
**agent** (C-44, §4.11).

### 1.5 This phase: the next level

The realignment (Stages 0–3) rebuilt the core and closed the first loop:
a fast language core, a git-backed System State, a Studio that edits it by hand
and an Assistant that edits it through the same operations. This phase takes
Agentique from "works" to **a product the Operator prefers to use every day**,
and makes it about what its name promises: **agentic systems**.

"Next level" means four outcomes, with quality as the headline:

1. **A world-class Studio.** Linear- and Figma-grade on every path the Operator
   uses daily; rarely used paths are clean but plain (C-32). Smooth at 10,000
   elements (C-33). Many quality-of-life features one expects from a daily tool
   (§3.4).
2. **A real agentic Assistant, independent of any one provider.** It runs on
   rig for every provider (C-34): Anthropic, OpenAI, OpenRouter and DeepSeek in
   this phase (C-35). It maps a task onto the architecture, shows a plan, thinks visibly,
   acts through tools, checks its work, follows skills and keeps notes the
   Operator approved. It works long tasks that the Operator can follow, steer
   and interrupt, under one of three autonomy modes (C-38 to C-41).
3. **A Settings view as good as the best.** Providers and keys, models, the
   Assistant's behaviour, appearance, keyboard and projects, with the patterns
   of GitHub, VS Code, Zed and Raycast (§3.7).
4. **Agentic systems.** Systems designed in Agentique can contain **agents**:
   parts whose behaviour is produced by an AI model, fast ("system 1") or
   deliberate ("system 2") (C-42). Agents are designed on the Surface (Stage 6),
   simulated with deterministic stand-ins and recordings (Stage 7, C-43), and
   implemented in the project's own code, linked and checked (Stage 8, C-46).

The phase also **completes Scenario A**: simulation (Stage 7) and implementation
links (Stage 8) close the first proof (C-19).

**Ambition in the outcome, simplicity in the mechanism.** Every item in this
document names the scenario step, decision or recommendation it serves (§8.1).
The discipline of §1.3 applies to Agentique's own design first.

### 1.6 What Agentique is not (C-14, extended)

The first seven rows are the original C-14. The rows after them follow from the
decisions named in each row.

| Non-goal | Horizon |
|---|---|
| A certified or fully conformant KerML/SysML implementation; interchange with other SysML tools is not required | Not a goal. Completeness may grow where it pays off (C-4) |
| A tool where users read or write SysML text; SysML sits entirely beneath the Surface and the agents | Never |
| A multi-user, collaborative or cloud-hosted service | Not now; one Operator on one machine |
| The home of a project's source code or deployments | Never; Agentique links to them |
| An autonomous multi-agent factory | Not yet; one Assistant first, orchestration later (C-5) |
| A physics or physical-systems simulator | Not the aim; simulation tests architecture and contracts (C-16) |
| A generator of verification paperwork for its own sake | Never; proof is working software the Operator uses (C-15) |
| An agent framework or runtime for other people's code | Not a goal; agents in designed systems are implemented in the project's own code (C-46) |
| A chat product with the architecture on the side | Never; the Surface stays an equal way to work (C-3) |
| Local models and Gemini | Not in this phase (C-35); the provider layer keeps the door open (Q-12) |
| Typed fast-decision APIs other than Jev | Not in this phase; Jev is a model provider for fast agents (C-35, Q-11) |
| Spending limits | Not in this phase; costs are shown, not capped (C-37) |

---

## 2. Intended experience

The scenarios describe the experience Agentique must reach, in the order its
capabilities are built (C-17). Once a capability exists, it is available at any
time in a project's life. **Using Agentique is a continuous loop, not a
pipeline** (C-17): architecture changes after code exists, the new architecture
is simulated, and it is then re-implemented.

Each table lists what the Operator does, the observable success, and the
behaviour on failure and recovery. A stage is accepted when the Operator has
done its steps and is convinced (C-15).

### 2.1 Scenario A: first proof, a small new system (C-18, C-19)

The example system is a **URL shortener** with an API, storage and click
statistics (`models/url-shortener/`). Any comparably small software system will
do.

| Step | What the Operator does | Observable success | Failure and recovery behaviour |
|---|---|---|---|
| A1 | Opens Agentique, creates an empty project and tells the Assistant the idea in ordinary language. | The project opens quickly on a fresh machine with no special install step beyond the app and a key for one supported provider. The Assistant replies in the Conversation. | A missing or invalid key produces a clear message that links to the setting that fixes it. The Surface still works fully by hand. |
| A2 | Watches the Assistant build the architecture. | Components, interfaces, connections and requirements **appear on the Surface as they are created**. Each Assistant action is visible in the Conversation as it happens (C-8). Names are standard and plain. | The Assistant asks when it meets a major decision (for example a separate statistics service or not) instead of guessing (C-6). The Operator can stop it mid-work, and partial work stays visible and can be undone. |
| A3 | Explores and adjusts the architecture both visually and in conversation until satisfied. | Direct edits on the Surface (create, connect, rename, move, delete, set properties) and edits made through conversation have the same effect on the same System State. Invalid combinations are flagged at the element concerned, in plain language. | An invalid edit never silently corrupts the state: it is shown as invalid and can be fixed or undone (C-25). |
| A4 | Locks the parts they consider settled. | Locked parts are visibly marked. | The Assistant cannot change a locked part without asking (C-11). |
| A5 | *(Stage 7)* Defines and runs simulation scenarios, such as "create a link, then resolve it, then read the stats". | The scenario steps through the modelled parts and interfaces on the Surface. It reports clearly whether the architecture can carry it out and whether the requirements hold. | An impossible step (missing interface, wrong item type, unhandled message) is shown at the exact element, with no guessed outcome. A finished run is not reported as a passed check. |
| A6 | *(Stage 8)* Asks the Assistant to implement the system. | The code is written in an ordinary external git repository. Each component is linked to its code and tests. Agentique shows per component whether it is implemented and whether its checks pass. | Code that contradicts the model (for example a missing interface operation or a forbidden dependency) is shown as **drift** on the affected part. |
| A7 | *(Stage 8)* Runs the resulting system and tries it. | The URL shortener works as designed. | — |
| A8 | Days later, brings a loosely worded new idea: "add expiring links". | The Assistant first maps the idea onto the architecture and **shows what would change** before changing it. Unaffected parts are untouched. | If a locked part must change, the Assistant asks and explains why; the Operator confirms or refuses. A refusal leaves everything as it was. |
| A9 | Closes Agentique and reopens it later. | Everything is exactly as left: architecture, locks, history, conversation and links. History is browsable, with a visual "what changed" view between points in time. | After a crash, at most the work since the last save is lost, and the state is never corrupted. |

**Stage mapping.** A1–A4, A8 (at architecture level) and A9 were built in
Stage 3 and are accepted live in Stage 4 (C-29). A5 is Stage 7. A6–A7 are
Stage 8, which completes Scenario A (C-19).

### 2.2 Live acceptance of Scenario A (Stage 4)

Stage 3 has only run against a scripted stand-in model. This is the protocol
for running it live and accepting Stages 0–3 (C-29).

| Step | What the Operator does | Observable success | Failure and recovery behaviour |
|---|---|---|---|
| L1 | Sets a key for a provider the Assistant supports (for example `DEEPSEEK_API_KEY` or `ANTHROPIC_API_KEY`; the environment variable until Settings exists in Stage 5) and creates a fresh project. | The Conversation shows the model in use; no key banner. | A refused key is explained plainly; the Surface works by hand. |
| L2 | Runs A1–A2: describes the URL shortener in ordinary words. | Parts, ports, interfaces and requirements appear live as cards and on the Surface; names are plain; readable thinking summaries show between steps (R-31). The Assistant never shows SysML text (C-4). | Anything the model does against the skills (guessing a major decision, inventing names, showing SysML) is recorded as a failed task in the evaluation set (R-19), the skill is fixed, and the step is repeated. |
| L3 | Runs A3 and A4: adjusts by hand and in words; locks the settled parts. | Both kinds of edit behave the same; locks are marked. | An invalid edit shows at the element; undo works. |
| L4 | Runs A8: "add expiring links". | The Assistant says what would change first; before touching a locked part it explains and the lock prompt asks. | A refusal leaves everything as it was; the Assistant says what remains undone. |
| L5 | Stops the Assistant mid-work; undoes its turn. | Stop takes effect within about 50 ms; partial work stays until undone; undo restores the state before the turn. | — |
| L6 | Runs A9: closes, reopens; kills the process after an edit and reopens. | Model, locks, history and conversation as left; at most the edit being saved is lost. | — |
| L7 | Reviews the evaluation set's results (20–30 Scenario A tasks, three trials each, R-19) and the session's token cost. | Pass rates per task; must-hold behaviours (never claims an unconfirmed change; never changes a lock without confirmation) pass in all trials. The Operator accepts Stages 0–3 or lists what fails. | Failures become Stage 4 work items before anything new builds on them. |

### 2.3 Scenario D: daily use of a polished Studio

The Operator uses the Studio as their daily tool for architecture work
(Stages 5 onward). Budgets are in §3.3.

| Step | What the Operator does | Observable success | Failure and recovery behaviour |
|---|---|---|---|
| D1 | Starts Agentique. | First interactive frame in 400 ms or less on a warm start. The last project, camera and panel layout are restored. The start screen lists recent projects. | After a crash, a short note says what was recovered. A project open in another window is refused with a plain message. |
| D2 | Orients: fits the view (Shift+1), zooms to the selection (Shift+2), goes to an element by name (Ctrl+P), collapses containers, switches between the Architecture, Graph and Requirements views. | The camera moves in about 300 ms (instantly with reduced motion). Labels never clip into illegibility; levels of detail are tuned so the model stays readable. | A name with no match shows the nearest names. |
| D3 | Edits by hand: creates, connects by dragging from port to port, renames inline, moves, sets properties in the Inspector. | Each edit appears on the Surface within 16.7 ms at Scenario A size (§3.3). While dragging a connection, compatible ports are marked; a refused connection says why (Unreal Blueprint [57]). | An invalid edit is applied and shown at the element (C-25); Ctrl+Z undoes it. |
| D4 | Uses the command palette and shortcuts for everything. | Every action is in the palette with its shortcut; recent actions come first; `?` shows all shortcuts. | — |
| D5 | Reviews: the Problems panel, History with checkpoints and the "what changed" overlay, and "Colour by…" overlays (locks, problems, changed since a checkpoint). | Overlays have a legend; every row jumps to its element. | — |
| D6 | Arranges the workspace: resizes and collapses docked panels, uses a focus mode that hides them, follows the Windows theme. | The layout is remembered per project; panels never cover the Surface. | — |
| D7 | Works on a large model (10,000 elements). | Pan and zoom stay smooth (p95 frame time 16.7 ms or less); an edit reaches the Surface in 100 ms or less; the overview stays readable through levels of detail and a minimap. | — |
| D8 | Asks the Assistant something small while working. | The Conversation opens beside the Surface without covering it; the answer streams. | Provider errors are explained; the message is never lost. |
| D9 | Checkpoints (Ctrl+S) with a message and closes. | No prompts on close; nothing lost (A9). | — |

### 2.4 Scenario E: setting up providers and models in Settings (Stage 5)

| Step | What the Operator does | Observable success | Failure and recovery behaviour |
|---|---|---|---|
| E1 | Starts Agentique for the first time. | A three-step welcome: what Agentique is, connect a model provider (or skip), create or open a project; the URL shortener is offered as a sample. | Skip is always possible; the Surface works fully by hand (§4.2). |
| E2 | Opens Settings (Ctrl+,) › Providers › Anthropic, pastes a key and presses Test. | "Key works" within about two seconds. The model list appears with context size, capability badges and a list price marked as an estimate. | 401 "Key refused", 403 "Key has no access", 429 "Rate limited, try later", or "Could not reach Anthropic". The key is not saved unless the Operator chooses "Save anyway (offline)". |
| E3 | Saves. | The row says "Saved in Windows Credential Manager". The key is never shown again, only a hint such as `sk-ant-…a1B2` (R-25). | If the credential store is unavailable, a plain message says so and points to the environment variable. There is no plain-text fallback. |
| E4 | Adds OpenAI, OpenRouter, DeepSeek and TypeSafe AI (Jev) the same way. | Each provider row shows its own status and test result. | As in E2–E3. |
| E5 | Chooses the Assistant's default model and effort. | Effort levels the model lacks are disabled with the reason; models that cannot use tools are greyed out with the reason; missing features (for example "no prompt caching") are shown as badges (§4.8). | — |
| E6 | Switches the model for one conversation from the Conversation's composer. | The switch is instant; a one-line note says the prompt cache restarts. | A provider without a key links straight to its Settings row. |
| E7 | Searches Settings for "dark" and then for "api key". | Matching rows are shown with the match highlighted; searching matches descriptions and synonyms, not only labels; Esc clears. | — |
| E8 | Sets `ANTHROPIC_API_KEY` in the environment and restarts. | The row says "From environment variable ANTHROPIC_API_KEY (the saved key is ignored)" and editing is disabled. | — |
| E9 | Removes a key in the Danger zone. | A simple confirmation with Cancel focused; the credential is deleted. | A conversation using that provider shows a notice linking to the row. |

### 2.5 Scenario F: a long task with the agentic Assistant (Stage 6)

| Step | What the Operator does | Observable success | Failure and recovery behaviour |
|---|---|---|---|
| F1 | In the default autonomy mode ("Ask on major decisions"), asks: "add expiring links, rate limiting and an admin API". | The message is acknowledged within 100 ms. The Assistant first maps the task onto the architecture (reading tools shown as compact cards), then posts a **plan card** of 4–7 short steps (R-30). | A provider error is explained plainly; the message is kept; Retry is offered. |
| F2 | Watches it work. | Each step shows a collapsed thinking summary, live tool cards and the changes highlighted on the Surface. Plan steps tick off as they finish. Follow mode moves the camera to what changes. | — |
| F3 | Answers a major-decision question: "a separate admin service, or part of the API?" | The question card offers the options; the proposed change for the chosen option can be previewed on the Surface before the Operator answers (R-29). | The turn waits for the answer; the Operator can answer by typing, or stop the turn. |
| F4 | Types a correction while it works: "use a sliding window for rate limiting". | The message is shown as queued and delivered at the next tool boundary, inside the same turn; the plan updates (R-32). | A queued message can be taken back before delivery. |
| F5 | The task needs the locked API to change. | The lock card shows the change and the locked elements before and after, with Allow and Refuse and an optional note (C-11). | Refuse: the Assistant continues without that change and says what remains undone. |
| F6 | Presses "Stop and send" to redirect. | The current step stops within about 50 ms; the new message is sent; partial work stays and can be undone. | — |
| F7 | The turn ends. | A turn summary lists what changed (as links), problems before and after, tokens and estimated cost (C-37). | — |
| F8 | Undoes the whole turn, or single changes. | "Undo the Assistant's changes" restores the state before the turn; single changes can be undone from the review (should, §3.4). | If the Operator changed things since, the undo says so (as today). |
| F9 | Switches the conversation to "Ask before every change" and asks for another change. | Each change is previewed on the Surface as a "what changed" overlay before it applies, with Allow and Refuse (C-38). | — |
| F10 | Keeps working in the same conversation for a long time. | Past about 120,000 tokens the conversation is compacted between turns; a divider shows the summary; decisions and answers survive (R-33). | If a summary drops a decision, the Operator restates it; the evaluation set covers this (A-12). |
| F11 | The Assistant proposes a note: "the Operator prefers REST over gRPC". | A note card with Accept, Edit and Reject. Accepted notes are listed in Settings and used in later conversations. Nothing is remembered silently (C-40). | Rejected notes are never stored. |
| F12 | Switches to "Ask only on locks" for a routine batch. | The Assistant works without questions except at locks. | — |
| F13 | Adds their own skill file ("naming conventions") and asks for a change. | The skill is listed in Settings; a compact card shows when the Assistant reads it (C-41). | A malformed skill file is listed with its error and ignored. |

### 2.6 Scenario G: designing, simulating and implementing a system with agents

The URL shortener gains **link screening**: an agent checks each new link for
abuse, with a deterministic fallback.

| Step | What the Operator does | Observable success | Failure and recovery behaviour |
|---|---|---|---|
| G1 | *(Stage 6)* Asks: "screen new links for abuse with a fast AI check; if it fails, fall back to a blocklist". | The Assistant creates a shared contract `Screening` (ports: link in, verdict out), an agent `LinkScreening` that specialises `Screening` and `Agent` in fast mode, a deterministic `BlocklistScreening` as its fallback, and a guardrail requirement ("a suspicious link is never activated without review") with the agent as subject; it connects the agent to the API. | The Assistant asks on major decisions (for example: block suspicious links or hold them for review). |
| G2 | *(Stage 6)* Looks at it on the Surface and in the Inspector. | The agent card carries an agent badge and its mode; its fallback and guardrail are visible; the Inspector shows mode, model, confidence threshold and budgets (§4.11). | A fallback that does not share the agent's contract is a validation error at the agent. |
| G3 | *(Stage 6)* Opens Agentique's own architecture. | The Assistant appears as an agent in Agentique's self-model, with its tools as ports to the System State and its guardrails as requirements (C-44). | — |
| G4 | *(Stage 7)* Simulates "create a link" with the agent stubbed. | The scenario steps through the parts on the Surface; the agent's verdict comes from a deterministic stand-in; requirement checks are reported separately from run completion. | — |
| G5 | *(Stage 7)* Injects failures: timeout, invalid output, low confidence, refusal. | The run takes the fallback path; the guardrail holds; each switch is visible in the trace. | An agent without a fallback shows the failure at the agent; no outcome is guessed. |
| G6 | *(Stage 7)* Replays recorded real answers, and runs a live evaluation of 50 sample links apart from simulation. | Replay is deterministic. A live evaluation reports a pass rate over several samples and its cost; a good run can be kept as recordings (C-43). | A missing recording stops the run with the reason `missing-recording`; it never falls through to a live call. |
| G7 | *(Stage 8)* Asks the Assistant to implement the system, including the agent. | The code lives in the project's repository; the agent is linked to its code, instructions and recordings (C-46). | — |
| G8 | *(Stage 8)* Looks at the agent's checks. | Inputs and outputs match its ports; the fallback path exists; guardrail tests pass on recordings; the live pass rate meets its requirement. | A failing check is shown as drift at the agent. |
| G9 | *(Stage 8)* Changes the code so the agent drops its fallback. | Drift appears at the agent. | Fixing the code or the model clears it. |

### 2.7 Scenario B: a larger system (later, C-18)

The same journey applies to a system with many components and several
interacting subsystems. What it adds:

- The Surface stays **readable and fast** at that size through focus, grouping,
  levels of detail and search. (Stage 5 meets the frame and edit budgets at
  10,000 elements; readable overviews of large models are Scenario B work.)
- The Assistant keeps concepts **general** instead of multiplying
  near-duplicate parts. This is the "shallow concepts" failure (C-10).
- Several ideas can be explored on **branches** and compared visually before
  one is kept (C-24).

### 2.8 Scenario C: Agentique builds Agentique (later, C-13, C-18)

Agentique's own architecture (§4.6) is opened in Agentique. The Operator
explores it, and the Assistant makes a real change to Agentique, with its code
linked and checked (Stage 8 capabilities). Its locked core must stay intact.
This is also the proof that Agentique can take on an existing, messy codebase:
reading an existing codebase into a model before switching to "model as
contract" (Q-7).

---

## 3. Quality bar

Agentique is judged as a product the Operator wants to use every day (C-21).
The bar for this phase is **Linear- and Figma-grade on the paths the Operator
uses daily** (C-32): starting, orienting, editing on the Surface, the
Inspector and other panels, the command palette, the Conversation, Settings and
the first run. Rarely used paths are clean and consistent but plain.

### 3.1 How quality is judged

- **The Operator judges** (C-15). Screenshots, scripted journeys and numbers
  support that judgment; they never replace it.
- **At the end of each Studio stage**, the Operator reviews the daily paths
  side by side with reference screenshots of Linear, Zed and Figma, and with the
  previous stage's screenshots. The journeys (`a-build`, `a-reopen`,
  `a-assistant`, and new `d-daily`, `e-settings`, `f-long-task`) write their
  screenshots to a folder of the Operator's choice outside the repository.
- **A component gallery** (`--fixture components`) shows every token and
  component in both themes, so drift in the design system is visible in one
  place (R-26).
- **Blind comparison** decides the toolkit (S4.1): the Operator scores text,
  motion and "does not look like a widget toolkit" without knowing which build
  is which.
- **What the Operator checks** on each review: nothing clipped or overlapping;
  one spacing grid; one type scale; states (hover, press, focus, disabled,
  selected, changed) look the same everywhere; motion is short and purposeful;
  nothing shows SysML text; every error says what to do next.
- **Numbers** (§3.3) are measured on the Operator's machine (the reference
  machine) and in CI. They gate merges; they do not prove quality.

### 3.2 Design system

One design system serves the Surface, the Panels, the Conversation and
Settings. Starting values come from the design research; they are tuned with
the Operator on real models, in one place (R-26).

**Principles**

1. **The Surface leads; chrome recedes.** Neutral panels, scarce accent colour,
   fewer and smaller icons. Linear's 2024–2026 redesigns moved the same way:
   dimmer chrome, less saturated neutrals, fewer icons [49][50].
2. **Keyboard first and instant.** Every action is reachable from the command
   palette with its shortcut shown; keyboard-invoked UI does not animate
   [53][68].
3. **Every change is visible, attributable and reversible.** Change marks by
   actor, Assistant presence, turn review and undo.
4. **One state vocabulary** across Surface, Panels and Conversation: pending,
   running, waiting for you, done, failed, refused (after the Agent Client
   Protocol's tool-call statuses and Linear's agent guidelines [62][52]).
5. **Speed is a feature.** Frame and latency budgets are part of the design
   (§3.3), as Zed and Figma treat them [61][55].

**Tokens** (starting values; one Rust module; nothing hard-codes a value that
a token covers)

| Category | Tokens and starting values | Source |
|---|---|---|
| Type | Inter for UI (Inter Display from 20 px), a monospace face for names and values. Scale 11 / 12 / 13 / 14 / 16 / 20 / 28 px; weights 400 / 500 / 600; 13 px default UI text; 14 px Conversation prose; tabular numbers in the Inspector | Windows typography guidance [65]; design research. Inter versus Segoe UI Variable is decided side by side (Q-17) |
| Spacing | 4 px grid: 0, 2, 4, 6, 8, 12, 16, 20, 24, 32, 40, 48, 64. Rows 24 / 28 / 32 (compact, default, comfortable); controls 24 / 28 / 32; panel padding 12 | Primer size primitives [66] |
| Radii | 2 (tags), 4 (buttons, inputs), 6 (cards on the Surface), 8 (menus), 12 (palette, dialogs), full (pills, ports) | design research |
| Colour | Neutral 12-step scale plus five roles: accent (selection, focus), success (added, passed), warning (lock needing confirmation, drift), danger (validation error, removed), info (the Assistant's activity). Steps follow Radix's roles (backgrounds 1–5, borders 6–8, solid 9–10, text 11–12); text never below step 11 for body copy; connection lines at step 9 or above for 3:1 contrast | Radix Colors [63]; contrast computed in the design research |
| Elevation | Dark theme: a lighter surface step plus a 1 px border; shadows only for menus, popovers, the palette, dialogs and a dragged card | Apple's dark-mode guidance [104]; design research |
| Motion | 0 ms for keyboard-opened UI and frequent actions; 100 ms press and hover-in; 150 ms hover fade-out; 200 ms panels and expand; 300 ms camera moves; Fluent easing curves; reduced motion turns moves into instant jumps with fades of 100 ms or less | Fluent 2 motion tokens [64]; Linear's hover fade-out [51]; Kowalski and Freiberg on frequent actions [68] |
| Change highlight | Hold about 1.5 s, fade about 0.4 s; colour by actor (Operator: accent tint; Assistant: info) | proposal, tuned with the Operator |

**Theming.** Dark first, with a coherent light theme and a high-contrast
theme. All three are generated from three inputs (base, accent, contrast),
as Linear generates its themes in LCH, so they cannot drift apart [49]. The
theme follows Windows by default (Settings can override it). Windows 10 has no
Mica, so window backgrounds are plain colours.

**Iconography.** One open icon set with a permissive licence, drawn from
vectors at 16 px (12 and 20 px variants), used sparingly: toolbar, element
kinds in lists, card badges, state marks. The set is chosen in W5.2 after a
licence check.

**Components** (the gallery shows each in every state)

- **Shell:** title area with project name and branch; a slim toolbar;
  status bar (save state, problems, locks, background work; frame time in
  developer mode); docked, resizable, collapsible panels with splitters; a
  focus mode that hides them (Figma found floating panels slowed people down,
  so panels dock [54]).
- **Surface:** dot grid that fades with zoom [56]; card (kind, name, type,
  lock and problem badges, agent badge); port (shape shows direction; hollow
  when unconnected, filled when connected [57]); orthogonal edge with arrowheads
  by kind and label pills; container (expanded, or collapsed with boundary
  ports); selection ring drawn outside the card; marquee; alignment guides;
  change marks and removal ghosts; minimap; zoom controls; "Colour by…" overlay
  with legend (IcePanel tags [59]); named level-of-detail tiers with explicit
  zoom thresholds (Unreal's graph LOD, ComfyUI's low-detail threshold [57][58]).
- **Panels:** Outline; Inspector (label and value rows, inline validation);
  Requirements; Problems; History (checkpoints and "what changed").
- **Command:** command palette with groups, keywords, shortcuts, recent items,
  empty and loading states (Raycast's action panel: Enter runs the primary
  action, every action lists its shortcut [53]); context menu with the same
  actions; "go to element"; shortcut help (`?`).
- **Conversation:** Operator message; Assistant message (full width, streamed
  Markdown, element links); thinking row (collapsed summary); tool card by kind
  and status with a change chip [62][69]; plan card; question card with options
  and preview; lock card with before and after (Raycast's confirmation lists
  side effects [53]); note card; queued messages; turn summary (Warp groups a
  command's output as one block [60]); compaction divider; composer with
  context chips, autonomy mode and model picker, Send and Stop.
- **Primitives:** buttons (primary, secondary, ghost, danger; sizes 24, 28,
  32), icon button, text field and area, select, toggle, checkbox, radio,
  chip, badge, key cap, tooltip with shortcut, banner, inline message, toast
  (background events only), dialog, empty state, skeleton, spinner, progress,
  focus ring (2 px, outside, accent).

### 3.3 Performance budgets

Numeric budgets for this phase (C-33 for the Surface; R-27 for the rest). The
baseline was measured on 2026-09-27 on the reference machine: Windows 10,
RTX 3060 Ti (wgpu chose Vulkan), about 165 Hz display, release build of `main`
at `6fc90b78`, single runs unless noted (§5.2).

"Frame time" means the p95 of frame intervals during the scripted pan and zoom,
with no missed display frames, together with the CPU and GPU time per frame the
harness reports. On a display faster than 120 Hz the interval is limited by the
display, so the CPU and GPU times are checked too.

| Budget | Target | Baseline (measured unless marked) | How it is measured continuously |
|---|---|---|---|
| Frame time, pan and zoom, 1k elements | p95 ≤ 8.3 ms | interval p95 6.4 ms (display-limited); UI CPU p95 1.9 ms; GPU pass p95 0.17 ms: **met** | Stress harness (`--scenario stress`) asserts the budget on the reference machine; CI checks CPU-side ceilings |
| Frame time, pan and zoom, 10k elements | p95 ≤ 16.7 ms (C-33) | pan p95 34.1 ms, zoom p95 31.7 ms; UI CPU p95 21.9 ms; GPU pass p95 2.1 ms: **not met**, CPU-bound | As above |
| Input to next update | p95 ≤ 8.3 ms at 1k; ≤ 16.7 ms at 10k | pan p95 5.7 ms at 1k; 32.1 ms at 10k | Harness (existing `input_to_next_update_ms`); the toolkit spike adds input-to-present (S4.1, G2) |
| Start to first interactive frame | ≤ 400 ms warm; ≤ 1.5 s on the first start after a reboot | not measured directly: window after about 30–80 ms; two frames and exit 0.65–0.85 s warm, 1.44 s on the first start after a build | A start timestamp is added to the metrics report (W4.7); reference run |
| Project open to Surface shown | ≤ 100 ms at Scenario A size; ≤ 1 s at 10k | 2.6 ms open plus 0.1 ms scene at Scenario A size; 10k project not tried (a 10k scene build alone takes 2.7 s) | `project_timing` example and CI test ceilings; a 10k project fixture on the reference machine |
| Edit to Surface | ≤ 16.7 ms at Scenario A size; ≤ 50 ms at 1k; ≤ 100 ms at 10k (C-33) | derived: about 11 ms; about 120 ms; about 2.7 s (apply and save 5 ms, plus a full scene rebuild: 0.1 ms, 114 ms, 2,662 ms, plus one frame) | A journey step timing from apply to the first frame that shows it; CI ceilings on apply plus scene update |
| Send to first visible response | ≤ 100 ms to show the message and a "working" status; model output as it streams | not measured | Conversation journey timing |
| Conversation rendering | p95 frame ≤ 8.3 ms while streaming at 100 tokens per second; first paint of a reopened 200-message conversation ≤ 150 ms | a normal conversation rendered in under 0.5 ms per frame (Stage 2 prototype, `docs/stages.md`) | Conversation fixture in the harness (shared with S4.1, G3) |
| Memory | ≤ 300 MB private at Scenario A size; ≤ 450 MB at 10k; no growth over a 30-minute soak *(provisional until R-44)* | 351–353 MB private (322–325 MB working set) on the start screen and with URL shortener projects; 508–525 MB private at 10k | Harness reports peak private bytes; soak run at stage ends |

Feedback within 100 ms and a visible status for anything that takes more than
about 400 ms follow common responsiveness guidance [67].

**Where the budgets are enforced.**

- **CI (every pull request):** CPU-side work only, because the CI runner has no
  GPU: System State apply at 2k and 10k elements, scene update at 1k and 10k,
  layout and routing, label layout. Ceilings are set at about twice the target
  so noise does not fail builds, and tightened as the numbers settle. A budget
  not met yet (scene update at 1k and 10k until W5.5) has an interim ceiling
  at about three times today's reference measurement, guarding against
  regressions; the budget itself is unchanged.
- **Reference machine (before merging a change to the Studio or the Surface,
  and at every stage end):** the stress harness and journeys assert the frame,
  latency, start and memory budgets and exit non-zero on a miss. Reports go to a
  folder outside the repository and are never committed (§8.3).

### 3.4 Quality-of-life catalogue

Features one expects from a daily tool. **Must** means the stage that builds
the area (Stage 5 unless marked) is not accepted without it; **should** means
it is planned in that stage and dropped only with a reason recorded in
`docs/stages.md`; **later** means it waits for a scenario need. "Exists"
marks what the Studio already has.

| Feature | Tag | Reason |
|---|---|---|
| Command palette with every action, its shortcut and recent actions first (exists; extend) | must | Keyboard-first (principle 2); Raycast and Linear make the palette the map of the app [53] |
| Shortcuts for all frequent actions, and a `?` shortcut sheet | must | Learnability without documentation |
| Undo and redo everywhere, including panels (exists for the model) | must | Always recoverable (§4.2) |
| Shift+1 fit, Shift+2 zoom to selection, Shift+0 100%, Space+drag to pan | must | Standard muscle memory from Figma and tldraw [54] |
| "Go to element" by name (Ctrl+P), with fuzzy matching on qualified names | must | Fastest orientation in large models (D2) |
| Docked, resizable, collapsible panels, a focus mode, layout remembered per project | must | Today four fixed columns leave the Surface about 42% of a 1600 px window (§5.3) |
| Inline rename on the Surface (exists: double-click, F2) | must | Direct manipulation |
| Context menu on cards and edges with the palette's actions | must | Mouse users find actions where they point |
| Multi-select with bulk delete, move and lock | must | Routine editing |
| Problems panel with jump to element; counts in the status bar (partly exists) | must | Validity is visible where it matters |
| Autosave state in the status bar (exists: "Saved") | must | Trust in continuous saving |
| Recent projects and reopening the last project (exists) | must | D1 |
| Settings with search, deep links from errors, Ctrl+, | must | Scenario E; A1 |
| Selectable, copyable text across Conversation messages; copy message and code | must | Chat parity with modern AI products (§3.6); S4.1 gate G4 |
| Queued messages, Stop and send, Stop (Stop exists) | must (Stage 6) | Steering long tasks (C-39) |
| Empty states that teach: start screen, empty project, empty Conversation | must | First-run quality (E1) |
| Every error in plain words, with the action that fixes it | must | A1; Settings deep links |
| UI scale, high-DPI correctness, theme and reduced motion following Windows | must | Accessibility (§3.5) |
| Turn summary with problems before and after, tokens and estimated cost | must (Stage 6) | Visibility of Assistant work and cost (C-8, C-37) |
| Minimap on the Surface | should | Orientation at 10k elements (D7) |
| Follow mode: the camera goes to what the Assistant changes | should (Stage 6) | Ties the Conversation to the Surface (Zed's agent panel follows the agent [32]) |
| Per-change Keep or Undo review of an Assistant turn | should (Stage 6) | Reviewing is faster than re-prompting (Zed, Claude Code [32]) |
| "Colour by…" overlays: locks, problems, changed since a checkpoint; agents from Stage 6 | should | One model, many readings, instead of extra diagrams [59] |
| Alignment guides and snapping when moving cards | should | Tidy layouts without effort |
| Keyboard navigation between cards (arrow keys) with a visible focus ring | should | Keyboard-first; accessibility |
| Tooltips that show the shortcut | should | Learnability |
| Shortcut editor with a key recorder, conflict warning and per-binding reset | should | Settings › Keyboard; VS Code and Zed patterns [35][37] |
| A Windows notification or taskbar flash when a long turn finishes or waits for the Operator while the window is unfocused | should | Long tasks (Scenario F) |
| Export the Surface view as PNG or SVG | later | Sharing; no scenario asks for it yet |
| Several windows, or panels torn off into windows | later | One window is enough for one Operator now |
| Side-by-side comparison of two branches | later | Scenario B |
| Custom accent colour and density beyond compact and default | later | No need yet |
| Import and export of settings | later | One machine (C-9) |

### 3.5 Accessibility

- **Keyboard for everything**, with a visible 2 px focus ring outside the
  focused control.
- **Screen readers:** every control, panel, Conversation message, tool card and
  selected Surface element has an accessible name and role through AccessKit.
  The target is 18 of 20 checklist items read correctly by Windows Narrator
  (S4.1, gate G6). egui has no live regions yet [72]; announcements of streamed
  replies are listed as a known limit until it has.
- **Contrast:** body text at Radix step 11 or 12 on steps 1–3; lines and focus
  rings at 3:1 or more [63].
- **No colour-only meaning:** port direction is a shape, locks and problems
  have icons, agents have a badge.
- **Reduced motion** follows the Windows setting
  (`SPI_GETCLIENTAREAANIMATION` [48]) and can be overridden in Settings.
- **UI scale** from 100% to 200% without clipping; tested at 100%, 150% and
  200%.
- **Input methods:** IME composition in every text field (S4.1, gate G7).
- **High-contrast theme** (exists), generated from the same inputs as the
  others (§3.2).

### 3.6 The Conversation panel

The Conversation must match modern AI chat products (C-21). Stage 3 built
streamed Markdown, element links, tool cards, question prompts, Stop, Retry,
edit-and-resend, "Undo the Assistant's changes", a missing-key banner and
per-project history. This phase adds:

- **Selectable text across messages**, copy per message and per code block, a
  virtualised message list, Markdown tables (S4.1 decides how, gate G4).
- **Plan card** that ticks off live (R-30); **thinking rows** with collapsed
  summaries (R-31); tool cards typed by kind and status with change chips;
  **turn summary** with problems before and after, tokens and estimated cost.
- **Composer:** context chips (inserted selection), the conversation's
  autonomy mode, the model picker, Send, Stop, and while a turn runs, queue or
  "Stop and send" (R-32).
- **Preview:** questions and "Ask before every change" approvals show the
  proposed change on the Surface as a "what changed" overlay (R-29).
- **Note cards** with Accept, Edit and Reject (C-40); a **compaction divider**
  with the summary expandable (R-33).
- **The Assistant never shows SysML text.** It describes elements in words and
  links them by qualified name (C-4). The skills say so, and the evaluation set
  checks it (R-19). This was seen in the Stage 3 journey (§5.3).
- Errors (network, limits, invalid tool calls, refusals) are explained plainly
  and never lose the Operator's input; each links to the setting that fixes it
  when one exists.

### 3.7 The Settings view

Settings is a Studio view (Ctrl+,) with the patterns of the best settings
interfaces studied (R-24):

- **Navigation:** a left list of sections with a search box on top; content on
  the right in one column of at most about 720 px; the last section viewed is
  remembered. Every error the Operator can fix in Settings links straight to
  the row (VS Code and Zed deep-link to settings [35][37]).
- **Search** matches labels, descriptions and synonyms ("token" and "key" find
  the API key), highlights the match and clears with Esc. VS Code learned that
  people search in their own words [36].
- **Rows:** a label, a one-line description, the control on the right; rows
  that cannot be changed stay visible, disabled, with the reason ("Set by
  ANTHROPIC_API_KEY") [43].
- **Apply:** toggles, choices and sliders apply instantly; text commits on
  Enter or leaving the field, with inline validation that keeps the Operator's
  text. The API key uses its own small form (paste, Test, Save) and never mixes
  instant and saved controls in one form (Primer's rule [41]).
- **Defaults and reset:** only changed values are stored; a changed row has a
  marker and a reset control whose tooltip names the default.
- **Danger zone** at the bottom of Advanced, for removing saved keys and
  resetting all settings: buttons name the result, a simple confirmation has
  Cancel focused, and typed confirmation is reserved for irreversible,
  cascading actions (GitHub, Cloudscape [39][42]). Resetting all settings keeps a
  backup copy of the file first.

**Sections**

| Section | Contents | Tag |
|---|---|---|
| Providers | Anthropic, OpenAI, OpenRouter, DeepSeek, TypeSafe AI (Jev): key status, set, test, replace, remove; environment-variable override; model list with refresh, context size, capability badges and list price marked as an estimate (§4.8) | must |
| Assistant | Default provider and model; effort; default autonomy mode; thinking summaries on or off; skills (built-in, app-wide, per project; open folder; errors); notes (list, edit, delete); cost display per turn and per day | must (provider, model, effort, cost display); Stage 6 (autonomy mode, thinking summaries, skills, notes) |
| Appearance | Theme (use the Windows setting, light, dark, high contrast); UI scale; reduced motion (follow Windows, on, off); density (compact, default) | must (theme, scale, motion); should (density) |
| Keyboard | Searchable list of every shortcut; an editor with a key recorder, conflict warning naming the other command, and per-binding reset | must (list); should (editor) |
| Projects | Recent projects (remove from the list); default folder for new projects; per-project data (open folder, clear the conversation, notes and skills) | should |
| Advanced | Open the settings file; reset all settings; Danger zone | should |
| About | Version, licences, links | should |

---

## 4. Architecture

### 4.1 Responsibilities

| Actor | Responsibility | Not its responsibility |
|---|---|---|
| **Operator** | Intent, high-level decisions, approving changes to locked parts, approving notes, choosing the autonomy mode, judging quality | Low-level implementation |
| **Assistant** (later the Orchestrator) | Turning intent into changes to the System State through typed tools; mapping new ideas onto the architecture first; showing its plan; asking about major decisions (in the default mode); proposing notes; later, simulation and implementation | Silently changing locked parts; acting outside its tools; claiming results it did not observe; remembering anything the Operator has not approved |
| **Language core** | Meaning and validity of the System State under the chosen KerML/SysML subset, including the built-in `Agents` library (§4.11) | UI, AI transport, persistence format |
| **System State service** | Holding the live state; applying changes atomically; enforcing locks; publishing change events | Deciding intent |
| **History (git)** | Durable checkpoints, branches and past states | Live editing state |
| **Providers** (new) | Talking to model providers through rig; keys in the OS credential store; model lists and capabilities; usage and cost figures | Control over the System State; the Assistant's policy; being required for manual work |
| **Model providers** (Anthropic, OpenAI, OpenRouter, DeepSeek; TypeSafe AI's Jev for fast agents only) | Language reasoning behind the Assistant and, later, behind live evaluations of agents; Jev answers typed questions for fast agents and never serves the Assistant | Anything else |

### 4.2 Control and autonomy

- **One operation boundary.** The Surface and the Assistant change the System
  State through the **same typed operations**. Nothing bypasses them: no direct
  state or file mutation (C-2). New Assistant tools (plan, notes, skills) do not
  change the System State at all.
- **Three autonomy modes** (C-38), after the mode ladders of Claude Code and
  Codex [24][28], chosen per conversation and shown in the composer:

  | Mode | What happens before a change | Asks on major decisions | Locks |
  |---|---|---|---|
  | **Ask before every change** | Each change is previewed on the Surface as a "what changed" overlay and applies only after Allow | Yes | Always ask |
  | **Ask on major decisions** (default, R-29) | Changes apply as the Assistant works; every change is visible live and undoable | Yes, with a preview of the proposed change when useful | Always ask |
  | **Ask only on locks** | Changes apply as the Assistant works | No; it decides and says what it chose | Always ask |

  The executor enforces "Ask before every change"; the other two differ in the
  Assistant's instructions. What counts as a major decision is defined in the
  `decisions` skill and refined with the evaluation set (Q-3).
- **Locked parts** change only after the Operator's explicit confirmation of
  that specific change, in every mode (C-11). A lock covers the part and what it
  owns (R-11). Locks are stored in the System State and versioned with it.
  There is no "always allow" for locks.
- **Visibility instead of approval for everything** (C-8), except in "Ask before
  every change", which the Operator chooses.
- **Always recoverable.** The Operator can stop the Assistant at any time and
  undo its changes (R-12). A failed provider never blocks manual work.
- **The Assistant's output is untrusted input.** Tool calls are checked against
  their schemas and validated like any other change; the model is never the
  judge of what is valid. Text inside the model and inside tool results is data,
  not instructions. Notes the model proposes are stored only after the Operator
  accepts them (C-40).
- **Side effects outside the System State** (files and commands in a linked
  repository) arrive with implementation in Stage 8. How the autonomy modes
  extend to them is decided there (Q-15).

### 4.3 The role of KerML/SysML (C-4, C-20)

- **Foundation, not scripture.** Use enough KerML/SysML that Agentique and
  everything built with it rests on a stable core, standard parts and standard
  language. Don't read the specification as gospel or chase completeness.
- **A deliberate subset, grown by need.** `docs/subset.md` lists what is
  supported, partial and excluded, each with a reason (R-8). Add a construct
  only when a scenario needs it. This phase adds `enum def`, `enum` and
  enumeration values such as `AgentMode::fast` as feature values (for an
  agent's mode, Stage 6; today values are literals only), and the behaviour
  constructs simulation needs (Stage 7, Q-5).
- **Standard terms, standard meaning.** Deviations are recorded in one line
  each with the reason in `docs/deviations.md` (R-9). There are eleven today;
  this phase adds one for the built-in `Agents` library (§4.11).
- **Unsupported means explicit.** An unsupported construct is reported as
  unsupported, never silently approximated.
- **Invisible to users.** SysML text is the storage format for Agentique and
  its agents, never a user interface. The pinned OMG specifications and library
  bytes remain the reference.

Three claims stay apart:

| Claim | Status |
|---|---|
| **Standards conformance** (Agentique implements KerML/SysML correctly and completely) | Not a goal |
| **Model validity** (the System State is internally consistent under the supported subset: types exist, connections fit, parts compose) | Required |
| **Goal satisfaction** (the built system does what was intended) | The real target, shown by simulation, implementation checks, evaluations and the Operator's own use |

### 4.4 Model and code (C-7)

- **Working direction: "the model is the contract".** Implementation lives in
  external repositories. Model elements are linked to code, tests and (later)
  running services. Agentique checks that the code honours the model and reports
  differences as drift. Which checks give real protection is settled in Stage 8
  (Q-2), for ordinary parts and for agents (C-46).
- **Long-term goal: two-way reconciliation.** Either side may change, and
  Agentique proposes how to reconcile them.
- **Linking to deployed systems** is a later, separate kind of link:
  observation, not authoring.

### 4.5 History and persistence (C-24)

Confirmed on 2026-09-27 as built in Stage 2:

1. The System State is saved as **SysML text files** in `model/` plus
   `model/agentique.json` (identities, locks, next id), in a **git**
   repository embedded through `git2`, so no git install is needed.
2. By default the model folder lives inside the project's own code repository,
   so model changes and the code that implements them can share commits
   (Stage 8). A project's repository is used only if its working folder is the
   project folder; otherwise one is created there.
3. The live state lives in the running app and is **saved continuously**:
   atomic, crash-safe saves that write only the documents whose text changed.
   Commits happen at checkpoints. A checkpoint is refused while model files
   have been edited outside the app, so nothing the Operator changed by hand is
   overwritten or committed by surprise.
4. Short-range undo is Agentique's job. Longer-range history means git commits.
   Branches are for exploring ideas. Merges are done by element identity, never
   by git's text merge (not built yet).
5. The identity file format is
   `{"format": 1, "next": <id>, "elements": {"<id>": "<locator>"}, "locks": [...]}`.
   Locators are the kind and qualified path, with `#n` for the n-th unnamed
   member and `name#2` for a repeated name. Ids are never reused.
6. Deleting an element unbinds references to it by name, exactly as after
   reopening.

The persistence format is locked core (R-16). Nothing in this phase changes it.
Settings, keys, conversations, notes and skills are stored elsewhere (§4.9).

### 4.6 Agentique designed by its own principles; the self-model first

Everything Agentique promises other systems applies to Agentique first (C-20).
The self-model in `models/agentique/Agentique.sysml` is the contract for our own
crates, checked in CI by `tools/check_architecture.py` (R-15). **Changes that
cut across parts start there**, in the same change, and the check stays green.

**Self-model changes for this phase**, in the order they land:

| When | Change to `models/agentique/` | Why |
|---|---|---|
| Stage 4 (W4.8) | Add `part def Providers` as a planned part without a crate: "Talks to model providers through rig (and a thin Jev client, C-34); keys in the OS credential store; model lists, capabilities, usage". Add `dependency from Assistant to Providers` and `dependency from Studio to Providers` (Settings tests keys and lists models) | A distinct responsibility with three users over the phase (Studio's Settings, the Assistant, live evaluations of agents), and one place that contains rig's API churn (R-21) |
| Stage 4 (W4.8) | Extend the check: the rig crates, `tokio`, `reqwest` and the credential-store crates (`keyring`, `keyring-core`, `windows-native-keyring-store`) may only be dependencies of the Providers crate; `reqwest` in `agq-assistant` is listed as a temporary exception until W5.7, as the model marks other temporary dependencies | Provider neutrality and network isolation are checked, not hoped for (§8.7) |
| Stage 5 (W5.7) | Map `part 'agq-providers' : Crate;` into `Providers`; remove `reqwest` from `agq-assistant` | The crate exists |
| Stage 6 (W6.10) | Model the Assistant as the first agent: `part def Assistant :> Agents::Agent`, with ports for its tools towards the System State and requirements for its guardrails (locks need confirmation; output is untrusted; every change is visible and undoable) and no `fallback` part: when the Assistant fails, the Operator carries on by hand, which a requirement states (a failed provider never blocks manual work) (C-44) | Dogfooding (C-13, C-20); a real example of the concept |
| Stage 6 (W6.10) | Add the standard `dependency` relationship to the subset (a recorded locked-core decision) so that CI can validate `models/agentique/` with `agq-language`; `check_architecture.py` ignores constructs it does not need instead of rejecting them | Our own language core checks our own model (R-41) |
| Stage 7 | `Simulation` gets its crate; if live evaluations of agents need providers, add `dependency from Simulation to Providers` or a separate part (Q-20) | Decided with the simulation design |
| Stage 8 | `ImplementationLinks` gets its crate | — |

**Allowed dependencies after this phase** (arrows point inward; every edge
listed; not transitive):

```text
Studio → SystemState, Studio → Assistant, Studio → LanguageCore, Studio → Providers
Assistant → SystemState, Assistant → LanguageCore, Assistant → Providers
SystemState → LanguageCore, SystemState → History
Simulation → LanguageCore            (Stage 7 may add → SystemState, → Providers)
ImplementationLinks → SystemState
Providers → (nothing in Agentique)
LanguageCore → (nothing). No UI, network, async or AI types in LanguageCore,
SystemState or History.
```

**Locked core** (R-16): LanguageCore, the System State operations and the
persistence format change only with an explicit, recorded Operator decision.
This phase needs three such decisions, all in Stage 6: adding `enum def` and
enumeration values to the subset; adding the built-in `Agents` library (C-42
approves the approach; the exact text is confirmed at W6.9); and adding the
standard `dependency` relationship to the subset, so that our own core can
validate our own model (R-41).

### 4.7 Where rig sits and what it may touch (C-34)

rig (package `rig-core`, with `rig-agent` and the `rig` facade) is the
provider layer for **every** provider, Claude included (C-34). A provider that
released rig does not support, such as TypeSafe AI's Jev, gets a thin client
inside `agq-providers`, behind the same boundary and with the same small types,
until rig releases it (C-34, clarified 2026-09-27). Version 0.42.0
was released on 2026-08-17; releases are breaking 0.x versions every two to
three weeks, and `main` carried 110 unreleased, largely breaking commits on
2026-09-27 [1][4]. That churn shapes where rig may go.

- **Only `agq-providers` depends on rig.** rig's types never appear in its
  public API; the Assistant, the Studio and (later) simulation see Agentique's
  own small types: provider and model ids, model info with capabilities, a
  request, stream events (text, thinking summary, tool call start, tool input,
  usage), a reply, usage and a plain error (R-21). The API is synchronous: a
  request returns a handle that delivers events over a channel and can be
  cancelled; the async runtime stays inside the crate.
- **rig is pinned to an exact version** (`=0.4x.y`) and upgraded deliberately,
  one pull request per upgrade, with five tasks of the evaluation set run
  on each Assistant provider.
- **Our turn loop stays ours** (`crates/assistant/src/turn.rs`). It holds
  Agentique's policy (locks, questions, autonomy modes, steering, stop,
  compaction) and its 49 tests. It calls rig's per-provider completion models
  with streaming for each model call. rig's steppable `AgentRun` state machine
  [2] is evaluated in S4.2 and adopted only if it removes code without hiding
  that policy.
- **Async stays inside.** rig is async-only on tokio, with no blocking API
  [2]. `agq-providers` owns one background tokio runtime; the Studio stays
  immediate-mode and polls events each frame as it does today. Stop is a
  cancelled future (rig: drop the stream or call `cancel()` [2]), which also
  removes today's 50 ms polling.
- **TLS and build weight.** rig uses reqwest 0.13, whose `rustls` feature
  brings `aws-lc-rs`, a C library build [6]; rig also offers a `native-tls`
  feature [2]. S4.2 picks the TLS option that builds within this machine's disk
  limits. All of rig's roughly 25 providers are compiled in; there are no
  per-provider features (issue #2237 [5]).
- **Explicit settings always.** rig's agent defaults to one model call per run,
  and for unknown models such as `claude-opus-5` the Anthropic provider falls
  back to 2,048 output tokens unless set [2] (fixed on `main`, not released
  [4]).
  `agq-providers` always sets output limits and never relies on rig defaults.
- **Prefer upstream fixes to forks.** Where rig lacks something we need
  (below), the first choice is a contribution to rig; the second is a thin
  adapter inside `agq-providers`; a fork is a last resort that needs an
  Operator decision.
- **Churn in numbers.** rig's migration guide covers 0.30 to 0.42 in about
  3,900 lines [3]; its `pipeline` module was removed in 0.40 and `ToolDyn` was
  replaced [3]. Upgrades are therefore planned work, not background updates.
- **Alternatives that were considered.** `genai` is a thin multi-provider
  client without an agent loop; its escape hatch for extra request fields is
  documented for OpenAI-compatible payloads only, and whether it handles
  fallbacks and eager tool streaming was not verified [7]. There is no official
  Anthropic SDK for Rust [8]. Neither changes C-34.

**What Stage 3's Claude client does, and how it maps onto rig 0.42.0**

| Today (`crates/assistant/src/claude.rs`) | rig 0.42.0 | Plan |
|---|---|---|
| Adaptive thinking (`thinking: {type: "adaptive"}`) and `output_config.effort` | No typed API (issues #1452, #951 [5]); passed through `additional_params`, which rig flattens into the request body. Thinking passed this way is covered by rig's cassette tests for tool loops [2]; effort passed this way has no test (not verified) | Use `additional_params`; never combine with rig's own `output_schema`, which also writes `output_config` (R-22) |
| Thinking blocks stored and sent back unchanged | Thinking text, signatures and redacted blocks are preserved, but converted to rig's messages and back, not byte for byte [2] | Accept; conversation format 2 stores provider-neutral messages (R-23); S4.2 checks that Anthropic accepts the re-serialised history (P2) |
| Tool input streamed as generated (`eager_input_streaming: true`) | Not a field of rig's tool definition; raw tool JSON can be passed in `additional_params.tools`; malformed partial input becomes an in-band error that ends rig's agent stream [2] | Since our loop drives the completion model directly, send raw tool JSON; treat a malformed input as an error tool result (R-22) |
| Prompt caching: a breakpoint on the system prompt plus automatic caching of the conversation | Supported: `with_prompt_caching()` and `with_automatic_caching()`, with time-to-live options [2] | Use it |
| Server-side refusal fallbacks (`fallbacks: "default"` and the `server-side-fallback-2026-07-01` beta) | Request side possible (`additional_params`; the beta header is client-wide). Response side not supported: a `fallback` content block is an unknown type, which ends the stream with an error [2] | Decided (Q-18, overnight, pending the Operator's confirmation): a thin adapter in `agq-providers` removes the `fallback` block's frames before rig reads them and drops the declined model's reasoning and tool calls, as the hand-written client does; the upstream contribution is written as a proposal |
| Stop within about 50 ms, even while the network is silent | Dropping the future or `cancel()` stops at once; no run-scoped cancel handle yet (issue #2118 [5]) | Cancel the future (R-21) |
| Retries twice (1 s, 2 s, or `Retry-After` up to 10 s), only before any output | Nothing built in; `Retry-After` is readable from the error headers [2] | Our own retry wrapper in `agq-providers` with the same rule (R-22) |
| Token usage including cache writes and reads | Supported, including per-TTL cache writes and thinking tokens [2] | Use it |
| Stop reasons: refusal, max_tokens, tool_use, end_turn | Mapped (`ContentFilter`, `Length`, `ToolCalls`, `Stop`); rig's own agent would still run tool calls from a refused or truncated reply [2] | Our loop checks the stop reason before any tool runs, as today (R-22) |

### 4.8 Providers, capabilities and graceful degradation (C-35)

This phase supports **Anthropic, OpenAI, OpenRouter and DeepSeek** for the
Assistant, and **TypeSafe AI's Jev** for fast agents (C-35). Capabilities below
are those of rig 0.42.0, from its source and tests at tag `v0.42.0` (commit
`d5a34986`) [2], unless marked; the DeepSeek column also cites DeepSeek's API
documentation [105], and the Jev column (a thin client, not rig: C-34) cites
TypeSafe AI's [98]. "Not verified" means no source or test was found.

| Capability | Anthropic | OpenAI (Responses API) | OpenRouter | DeepSeek (`deepseek-flash`) | Jev (typed decisions) |
|---|---|---|---|---|---|
| Streaming text | yes | yes | yes | yes (rig's shared chat-completions path) [2] | not applicable: one typed answer per call [98] |
| Tool calling | yes | yes | yes | yes; a forced `tool_choice` is refused while thinking, and rig clears it [2][105] | not applicable |
| Tool arguments streamed as partial JSON | yes | yes | yes (shared chat-completions path) | yes: the first chunk of a call carries its id and name, later chunks its arguments [105] | not applicable |
| Parallel tool calls | not used: the Assistant runs tool calls in order by design; rig offers concurrency only in its own agent runner [2] | not used | not used | not used | not applicable |
| Reasoning control | partial: raw `thinking` and effort through `additional_params`; no typed API (issues #1452, #951 [5]) | yes: typed effort and summary settings | partial: raw `reasoning` parameters | partial: top-level `reasoning_effort` (`low`, `high`, `max`; default `high`; no `medium`) and `thinking: {type}` through `additional_params` [105] | not applicable |
| Reasoning text streamed | yes (summaries when requested [11]) | yes | yes | yes (`reasoning_content`, full text, not a summary) [2][105] | no |
| Reasoning preserved across tool turns | yes (normalised) | yes (encrypted reasoning) | yes (`reasoning_details` with signatures) | required: with tools, every earlier assistant turn's `reasoning_content` must be sent back or the API answers 400 [105]; rig attaches it to every assistant message with text or tool calls [2] | not applicable |
| Structured output, native | yes | yes | yes | no: `json_object` only; rig does not map `output_schema` [2][105] | yes, and only that: yes or no, a choice among up to 255 options, or a score on 2–10 levels, with probabilities and (for choice and score) a confidence [98] |
| Prompt caching control | yes (manual and automatic, time-to-live) | partial (automatic, cache key; no breakpoints, issue #2170 [5]) | partial (system prompt only) | automatic only, no control [105] | not applicable |
| Usage including cache counts | yes | yes | yes | yes (`prompt_cache_hit_tokens`, `prompt_cache_miss_tokens`; reasoning tokens separately) [2][105] | input and output tokens only; output is free [98] |
| Image input | yes | yes | yes | the model list offers it, but rig's path passes image parts through for DeepSeek to refuse [2]; not used | no |
| Server-side refusal fallbacks | no in rig (see §4.7); on `claude-opus-5` through the thin adapter in `agq-providers` (Q-18) | not applicable | not verified | not applicable | not applicable |
| Server-side compaction | not carried through rig (unknown blocks end the stream [2]) | not verified | not applicable | not applicable | not applicable |
| Model list | provider endpoint `GET /v1/models` [16]; not provided by rig (not verified) | not verified | public model list endpoint [100] | `GET /models`, with rig's model lister [2][105] | `GET /v1/models`; pinned versions such as `jev-1.13.0` are accepted though not listed [98] |

Other providers rig supports (Gemini, Ollama and local OpenAI-compatible
servers, Mistral, xAI, Groq and more [2]) wait for a scenario need (Q-12).

**Capabilities are data, not code paths.** `agq-providers` holds a small table
of capabilities per provider and model family, filled from the matrix above
and confirmed by five tasks of the evaluation set on each Assistant provider (A-8). The Assistant
asks the table, never the provider's name. When a capability is missing:

| Missing capability | Behaviour |
|---|---|
| Tool calling | The model is not offered for the Assistant; Settings shows it greyed with the reason |
| Reasoning control | The effort control is hidden for that model; the provider's default applies |
| Reasoning text | The thinking row shows "Thinking…" with the elapsed time and no summary |
| Tool arguments streamed | The tool card shows "Preparing change…" until the call is complete |
| Prompt caching | Works; the model row warns that long conversations cost more; the turn summary shows no cache share |
| Usage with cache counts | Tokens are shown without the cache split |
| Server-side refusal fallbacks | A refusal ends the turn with a plain notice and a "Retry with <model>" action; nothing of the refused reply runs |
| Server-side compaction | Not used for any provider: compaction is client-side (R-33) |
| A smaller context window | Compaction thresholds scale to the window (R-33) |
| Image input | Not used in this phase |

### 4.9 Where settings and secrets are stored

| What | Where | Format | Owner |
|---|---|---|---|
| Settings (Operator choices) | `%APPDATA%\Agentique\settings.json` | JSON with `"format": 1`, holding only changed values; watched, so hand edits apply live; never a secret | Studio |
| API keys | Windows Credential Manager: a generic credential per provider, target `agentique:<provider>`, persistence Local | Written through `keyring-core` with the Windows store [46]; at most 2,560 bytes per secret [47] | Providers |
| Session (last project, cameras, layouts) | `%APPDATA%\Agentique\studio-session.json` (exists) | JSON; theme, contrast and reduced motion move to settings | Studio |
| Per-project data | `%APPDATA%\Agentique\projects\<folder>-<hash>\` | `conversation.json` (format 2, R-23; today's `conversations\` files move here in W5.7), `notes.md`, `skills\<name>\SKILL.md` *(provisional for notes and skills, Q-10)* | Studio and Assistant |
| App-wide notes and skills | `%APPDATA%\Agentique\notes.md`, `%APPDATA%\Agentique\skills\<name>\SKILL.md` | Markdown; one note per `- ` line; skills in the Agent Skills format [21] | Assistant |
| The model | `<project>/model/` in git | Locked persistence format (§4.5) | History |

**Key handling contract** (R-25):

1. A non-empty environment variable (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`,
   `OPENROUTER_API_KEY`, `DEEPSEEK_API_KEY`, `TYPESAFE_API_KEY`) wins over the
   stored key, as in Zed and GitHub CLI
   [37][40]; Settings says so and disables editing.
2. Keys are written to the Credential Manager, never to a file, a log, the
   project folder or the System State. The Studio holds a key only in the input
   field while it is typed and clears it after Save.
3. **Test** calls an endpoint that needs the key and sends no prompt: the
   models endpoint for Anthropic [16] and OpenAI (not verified); for
   OpenRouter, whose model list is public [100], an authenticated endpoint
   chosen in W5.8 (not verified); for DeepSeek, `GET /user/balance` (rig's key
   check) or `GET /models` [105]; for Jev, `GET /v1/models` [98] (not verified
   whether it needs a key). The result maps to plain states (works,
   refused, no access, rate limited, cannot reach), as Jan's key test does [45]. Save runs the test first; "Save anyway (offline)" is
   explicit.
4. After Save the key is never shown again; Settings shows the fixed prefix and
   the last four characters, stored in the settings file.
5. If the store fails (no storage access, no logon session), say so and point to
   the environment variable. **No plain-text fallback** (GitHub CLI falls back
   to a file [40]; we do not).
6. One thread owns all credential access, with a timeout per call: the
   `keyring` store does not reliably order operations on one entry from
   different threads [46].

### 4.10 The Assistant: loop, tools, skills, notes, compaction

The Assistant stays one crate (`agq-assistant`) with its own turn loop (R-21).
Everything below still changes the System State only through `apply_changes`
and the Studio's executor, exactly like an Operator edit (§4.2).

**The loop.** Gather context, act, check, repeat, as the leading agent products
converge on [18][19]:

1. The Operator's message (with the autonomy mode as a short note, so changing
   the mode does not rebuild the prompt cache [13]) is added to the
   conversation.
2. For each model call: send the system prompt (core skills, the list of other
   skills, accepted notes), the tools and the conversation through Providers;
   stream text, thinking summaries, tool calls and usage to the Studio.
3. Check the stop reason before anything runs: a refused or truncated reply runs
   no tools. Check each tool call's input against its schema; invalid input gets
   an error result.
4. Hand valid calls to the executor in order. In "Ask before every change",
   `apply_changes` first shows its preview and waits for Allow. Locks always ask.
5. Add the results and any queued Operator messages (R-32) as one entry; repeat
   until the model finishes, the Operator stops or sends, a refusal, a cut-off,
   an error, or the 40-call pause (kept from Stage 3; OpenAI's Agents SDK stops
   at 10 turns and VS Code asks after 50 requests [34][102]).
6. After the turn: a turn summary (changes, problems before and after, tokens,
   estimated cost); compaction if the conversation is long (R-33).

**Tools.** Five exist; three are added. None but `apply_changes` changes the
System State.

| Tool | Purpose | Status |
|---|---|---|
| `read_model` | Read the model. Without an element it returns an **outline** (qualified names, kinds, locks, problem counts) under a token budget, not the whole text (R-34) | exists; outline added in Stage 4 |
| `find_elements` | Find elements by name or kind | exists |
| `get_problems` | Validation problems, optionally for one element | exists |
| `apply_changes` | One System State change (one undo step), prepared on a copy, applied by the Studio | exists |
| `ask_operator` | One question with two to four options; optional `preview` (a proposed change shown on the Surface) | exists; preview added in Stage 6 |
| `update_plan` | The plan card: 3–7 steps of a few words, statuses pending, running, done; one running. Used for tasks of three or more steps; re-sent after compaction (after Codex's `update_plan` [29]) | Stage 6 |
| `read_skill` | Read the body of a skill listed by name and description (progressive disclosure [21]) | Stage 6 |
| `propose_note` | Propose a short note (project or app-wide) for the Operator to accept, edit or reject | Stage 6 |

Every tool result is capped at about 8,000 tokens, with a message that says how
to narrow the request (Anthropic's guidance on tool results, which notes that
Claude Code caps tool responses at 25,000 tokens by default [20]).

**Skills** (C-41). Skills use the Agent Skills format: a folder with
`SKILL.md` holding a `name`, a `description` and a body [21]. The seven
built-in skills become such files, compiled in as defaults. The **core
skills** that define the Assistant (who it is, modelling, simplicity, ideas,
decisions, locks, tools) are always in the system prompt; other skills appear as
name and description, and their body is read with `read_skill` when relevant.
The Operator's skills live app-wide and per project (§4.9); a skill with the
same name overrides in the order project, app-wide, built-in. Settings lists
them with their source and any error (R-36).

**Notes** (C-40). The Assistant proposes a note with `propose_note`; the note
card offers Accept, Edit and Reject. Accepted notes are Markdown files (§4.9),
listed and editable in Settings › Assistant › Notes, and sent after the skills in
the system prompt, capped at about 2,000 tokens (Settings asks the Operator to
prune beyond it). Nothing is remembered silently: Gemini CLI's Auto Memory and
Devin's knowledge suggestions work the same way [30][103]. Silent memory
grows stale: Claude Code, for example, has its model shorten its memory file
as it nears the limit [27]. Architecture decisions belong in the
model (documentation, requirements), not in notes (R-35).

**Visible thinking** (R-31). The thinking row shows a collapsed summary per
step. On Anthropic models this needs `display: "summarized"`: on
`claude-opus-5` and newer the default is `"omitted"`, which is why Stage 3's
thinking stream carries no text [11]. On models that offer progress updates
(`display: "updates"`, for example `claude-opus-5-5` [11]) the update becomes the
subtitle of the next tool card. Other providers follow §4.8.

**Steering** (C-39, R-32). While a turn runs, Enter queues a message (shown
greyed, can be taken back); the loop delivers it with the next set of tool
results, inside the same turn. "Stop and send" stops the current step and sends
at once. This is the default in Claude Code, Cursor, Zed and Codex [25][31][32][29].
If the Operator edits the Surface during a turn, the next tool result names what
changed so the Assistant re-reads instead of acting on a stale model.

**Compaction** (R-33). Client-side and the same for every provider, because
Anthropic's server-side compaction blocks cannot pass through rig (§4.8). After
a turn, when the last request's input exceeded about 120,000 tokens or 60% of
the model's context window, whichever is smaller, one extra model call
summarises the conversation with Agentique's instructions: keep the Operator's
intent, every decision and answer verbatim, lock confirmations given or refused,
open questions, the plan and the names of changed elements; drop tool-result
bodies, because the System State can always be re-read. The next request starts
from the summary instead of editing earlier turns, so no thinking block is
altered. The Conversation keeps the full transcript for the Operator and shows a divider with the summary.
Mid-turn, past about 300,000 tokens or 85% of the window, the turn pauses and
compacts before continuing. (Codex compacts at 90% of the window and Gemini CLI
at 50% [29][30]; Anthropic's on-demand compaction lets the application choose
the moment, as here [14].)

**Evaluation** (R-19). A set of 20–30 tasks drawn from Scenario A and later
from F and G, each run three times, graded on the resulting System State by
code (expected elements, no new problems, locks untouched unless confirmed,
questions asked when a major decision was open, no SysML text shown), plus one
rubric grader for simplicity calibrated against the Operator's judgment.
Must-hold behaviours must pass in every trial (pass^3); capabilities are
reported as pass@3 [23]. Evaluations run on demand with a key and never in the
default `cargo test`. Task definitions are committed; results and transcripts
never are (§8.3). The same set compares models, effort levels and providers
before a default changes (Q-19).

### 4.11 Agents as model concepts (C-42)

An **agent** is a part whose behaviour is produced by an AI model. It covers
fast parts ("system 1": classify, route, score, extract) and deliberate parts
("system 2": plan, use tools). SysML v2 has no AI concept: a part may represent
software components, organisations or users (SysML 7.11.1 [82]); a kind gets
its meaning by specialising a library definition (SysML 7.6.8 [82]). So an
agent is a part definition that specialises a built-in library definition.

**The built-in library** (Stage 6, W6.9; exact text confirmed then under R-16):

```sysml
package Agents {
    doc /* Agentique's built-in library for agents (C-42). Not part of the
         * SysML standard library (deviation 12). */
    enum def AgentMode {
        doc /* fast: quick, cheap decisions (classify, route, score, extract).
             * deliberate: reasoning (plan, use tools, handle hard cases). */
        enum fast;
        enum deliberate;
    }
    abstract part def Agent {
        doc /* A part whose behaviour is produced by an AI model. Its inputs and
             * outputs are ports; the tools it may use are ports connected to the
             * parts that provide them; its guardrails are requirements whose
             * subject is the agent or its contract. model is the provider's
             * model id. Below minConfidence an answer goes to the fallback or
             * to a person. fallback is redefined by a deterministic part that
             * shares the agent's contract. */
        attribute mode : AgentMode;
        attribute model : ScalarValues::String [0..1];
        attribute minConfidence : ScalarValues::Real [0..1];
        attribute maxLatencyMs : ScalarValues::Natural [0..1];
        attribute maxCostPerCallUsd : ScalarValues::Real [0..1];
        part fallback [0..1];
    }
}
```

**How the rest is expressed**, all with constructs the subset supports today
(apart from `enum def` and enumeration values):

| Concern | Standard concept | Example (Scenario G) |
|---|---|---|
| The contract shared by the agent and its fallback | An abstract part def with the ports, specialised by both | `abstract part def Screening { port link : LinkIn; port verdict : VerdictOut; }` |
| The agent | A part def specialising the contract and `Agent` | `part def LinkScreening :> Screening, Agents::Agent { :>> mode = Agents::AgentMode::fast; part :>> fallback : BlocklistScreening; }` |
| The fallback | A deterministic part def specialising the same contract, redefining `fallback` | `part def BlocklistScreening :> Screening;` |
| Inputs and outputs | Ports with directed items | `port def LinkIn { in item link : ShortLink; }` |
| Tools it may use | Ports connected through interfaces to the parts that provide them | a port to a reputation service part |
| Guardrails | Requirements whose subject is the agent (or the contract), satisfied by the part that enforces them | "A suspicious link is never activated without review" |
| Mode, model, thresholds and budgets | The attributes above | `:>> minConfidence = 0.8;` |
| Instructions | The agent's `doc` summarises them; the full prompt lives in the implementation and is linked in Stage 8 | — |

**New validity rules** *(provisional, settled by S6.1)*:

- `wrong-fallback`: an agent's `fallback` must be typed by a part definition
  that specialises every part definition the agent specialises except `Agent`
  (the shared contract).
- `agent-fallback`: a fallback must not itself be an agent. This follows
  runtime assurance practice, where a simple, verified recovery function
  protects a complex one (Simplex; ASTM F3269 [85]; EASA's "traditional backup
  system" [86]; DARPA's work on assuring learning-enabled components [87]). Escalating from a fast agent to a deliberate one is a
  different thing: two agents and a routing part (Q-13).

**Why this form and not the others.** A library definition has instance-level
meaning, so it can be simulated, checked and made the subject of requirements.
The standard's keyword form (`#agent part def X;` through semantic metadata,
SysML 7.27.3–7.27.4 [82][84]) is shorthand for the same specialisation. It needs
metadata, `meta` expressions and implied specialisation, which the subset
excludes (deviation 2), so it can be added later without changing meaning. Plain
metadata (`@AI`) has "no instance-level semantics" (KerML 7.4.13 [83]) and
could not be simulated.

**Deviation 12** (recorded in W6.9): the `Agents` library is Agentique's own
built-in library, not part of the SysML standard library; like the other
built-in definitions, `Agent` does not specialise `Parts::Part` (deviation 2).

**On the Surface** (R-38): an agent card carries an agent badge and its mode.
It is marked by shape and badge, not by a new colour, so colour stays free for
state. Its fallback and guardrails are one click away. The Inspector has an
Agent section (mode, model, thresholds, budgets, fallback).

**Fast and deliberate modes in 2026.** Every major provider offers a fast,
cheap tier and a deliberate tier, and an effort control inside a model:

| Tier | Examples (list price per million input / output tokens) | Latency | Good for inside a running system |
|---|---|---|---|
| Fast | Claude Haiku 4.5 $1 / $5 [10]; GPT-6 Luna $0.10 / $0.50 [91]; Gemini 3.5 Flash-Lite $0.30 / $2.50 [92] | Haiku 4.5 about 0.68 s to first token [93]; Luna at low effort about 2.2 s [93] | Classification, routing, extraction, high-volume per-event decisions |
| Deliberate | Claude Opus 5.5 $4 / $20; Claude Fable 5.1 $10 / $50 [10]; GPT-6 Astra $10 / $50 [91] | Tens of seconds to minutes; Luna at maximum effort about 91 s, about 40 times its low-effort latency (indicative figures) [93] | Planning, novel or hard cases, multi-step tool use |
| Deterministic fallback | Code | Microseconds to milliseconds | The safety envelope, hard real time, recovery |

Robotics uses the same split by control rate: Figure's Helix runs a
vision-language model at 7–9 Hz over a 200 Hz control policy; NVIDIA's GR00T N1
runs 10 Hz over 120 Hz [96]. Research systems escalate from fast to slow with a
monitor (SOFAI [94]) or route and cascade by difficulty (RouteLLM, FrugalGPT
[95]).

**Jev** (the Operator's example) is a hosted model from TypeSafe AI, announced
on 2026-09-15, that answers typed questions about a given state: yes or no, a
choice among up to 255 options, or a score, with probabilities and a confidence
value; it produces no free text [97][98]. It is priced at $0.042 per million
input tokens with free output [98][100]. The vendor's latency claim of
70–500 ms per call was measured from its own laptops, and "0% hallucination" is
admitted to be "not empirical" [97]. The vendor's own evaluations show labels
flipping across runs [98], and it is not OpenAI-compatible [99]; OpenRouter serves it through a separate
route, not its chat API [100]. One
independent test found accuracy close to fast LLMs on intent routing (92.2%
versus 93.6%) at much lower cost [101]. It is exactly a fast-mode agent's shape
(a typed decision with a confidence), so C-35 makes it a model provider for
fast agents in designed systems (Q-11 resolved). rig's experimental
`rig-typesafeai` crate is not released: crates.io holds only a `0.0.0`
placeholder, published 2026-09-21 [9]. Until it is, `agq-providers` has a thin
client for Jev behind the same boundary (C-34). Jev is used for live
evaluations of fast agents (Stage 7) and examples (Stage 8), never for the
Assistant.

**Simulation of agents** (Stage 7, C-43, R-39). The agent's contract (ports,
guardrails, budgets) is checked in every mode; modes differ only in where the
agent's outputs come from, and the trace records which:

| Mode | Where outputs come from | Deterministic | For |
|---|---|---|---|
| Stubbed | A deterministic stand-in: schema-valid outputs or scripted answers per scenario, with **injected failures** (timeout, invalid output, low confidence, refusal, tool error) | Yes | The architecture itself: routing, ports, guardrails, fallback and human-approval paths; before any prompt or code exists (as Pydantic AI's `TestModel` and the OpenAI Agents SDK's scripted model do for code [88]) |
| Recorded | Replays of real answers, keyed by a digest of the canonical request (model, instructions, input, schemas, settings) | Yes | Realistic regression checks at no cost. A missing recording stops the run with the reason `missing-recording`; it never falls through to a live call (VCR's `none` mode [89]) |
| Live evaluation (apart from simulation) | The real model, several samples | No | Quality, real latency and cost, reported as pass rates; a good run can be kept as recordings. Temperature 0 does not make a model deterministic [90], and newer Claude models accept no temperature but 1.0 [17] |

Live evaluations are not simulation: they have external effects and are not
deterministic, so they run separately and never inside a simulation run
(§5.5).

**Implementation of agents** (Stage 8, C-46, R-40) *(provisional until Q-2)*: an
agent is ordinary code in the project's repository, using any provider SDK.
Agentique links the agent to its code, its instructions and its recordings, and
checks: the code's input and output types match the agent's ports; the fallback
path exists; guardrail tests pass on the recordings; the latest live
evaluation's pass rate meets the agent's requirement. A failing check is drift.

### 4.12 Quality expectations for the code

- **Simple before clever.** Pick the least complex design that meets the
  current stage's scenario. Build for the next stage only when that stage
  starts.
- **Stable core, experimental edges.** Experiments go in edge modules, behind
  flags or on branches, never in the language core or the System State
  operations.
- **One of each thing.** One language engine, one application, one persistence
  mechanism, one assistant layer, one provider layer, one design system, one
  settings table.
- **Readable by the Operator.** An engineer, or the Operator with an agent's
  help, can understand any part from its model element, its README and a short
  read of the code.
- **Tests prove behaviour, not ceremony.** Automated tests cover the language
  core, state operations, persistence, tool contracts and the provider layer
  (against recorded streams, without the network). Journeys test what the
  Operator uses.
- **Performance is part of correctness.** Budgets (§3.3) are checked
  continuously; a regression is a failing check, not a note.

---

## 5. Current-state alignment

### 5.1 What exists (evidence map)

State of `main` at `6fc90b78`. Line counts include tests unless noted.

| Area | What it is | Evidence | Actual state |
|---|---|---|---|
| Language core | The SysML subset as one element tree: parse, print, link, validate; built-in `ScalarValues`; no dependencies | `crates/language` (about 6,000 lines), `docs/subset.md`, `docs/deviations.md` (11 entries) | Works. Measured on the URL shortener (115 lines, 78 elements): parse 0.33 ms, validate 0.24 ms, rename plus revalidate plus print 0.38 ms. No behaviour constructs (Stage 7) |
| System State | Typed operations, atomic changes, locks, undo and redo, change events; `Project` ties it to History | `crates/system-state` (about 3,900 lines) | Works. Measured: open 2.56 ms, one change applied and saved 5.07 ms, checkpoint 14.0 ms, "what changed" 1.0 ms. One edit on 2,048 elements: 8 ms (Stage 2 record) |
| History | Model folder in git through `git2`; crash-safe saves; checkpoints; branches | `crates/history` (about 1,450 lines) | Works in the journeys. Merging branches by identity is not built; no branch UI |
| Assistant | Claude client (blocking `reqwest`, hand-written streaming), tool-use loop on a background thread, five tools, seven compiled-in skills, per-project conversation | `crates/assistant` (about 4,100 lines of Rust; `claude.rs` 633) | Tested only against a scripted stand-in and a local HTTP stand-in (49 tests). **Never run live.** Thinking text is always empty because `display` is not set [11]. `read_model` without an element returns the whole model text |
| Studio | egui 0.33.3 and wgpu 27 shell: Surface, Outline, Inspector, Requirements, History, Conversation, command palette, lock confirmation, change highlights | `crates/studio-native` (about 13,500 lines, of which about 2,400 are journeys, stress runs and tests) | All four journeys pass: `a-build` 74 steps, `a-crash` 5, `a-reopen` 15, `a-assistant` 19. On this machine wgpu chose Vulkan |
| Surface scene | Layout, routing, hit testing, culling; hierarchy, graph and requirements layouts | `crates/studio-scene` (about 5,900 lines) | Works. Every change rebuilds the whole scene (§5.2) |
| Renderer | Retained, instanced GPU pipeline; highlights fade on the GPU | `crates/studio-native/src/gpu.rs`, `scene.wgsl` | Not a bottleneck: GPU pass p95 0.17 ms at 1k, 2.1 ms at 10k |
| Design tokens | Type scale, spacing, radii, strokes, motion constants; Inter and JetBrains Mono | `crates/studio-native/src/theme.rs` | A start. No icons; hand-set colours; no generated themes |
| Settings | None. Key, model and effort come from environment variables; theme, contrast and reduced motion are in the session file | `crates/assistant/README.md`, `crates/studio-native/src/session.rs` | Missing |
| Self-model and check | Seven parts, crates mapped, explicit dependencies; a Python check in CI | `models/agentique/`, `tools/check_architecture.py` | Green. No Providers part; the check parses only the syntax the model uses |
| CI | fmt, clippy (also with `automation`), workspace tests, architecture check | `.github/workflows/ci.yml` | Green; about 17 minutes. No performance checks; no live evaluations |
| Governing documents | `REALIGNMENT.md`, `AGENTS.md`, `README.md`, `docs/stages.md` | repository root and `docs/` | Stages 0–3 "provisionally complete, pending Operator acceptance". This document replaces `REALIGNMENT.md` (C-47) |

### 5.2 Measured baselines (2026-09-27)

Reference machine: Windows 10 Home 19045, 12 logical processors, 16 GB RAM,
NVIDIA RTX 3060 Ti (driver 616.92; wgpu chose Vulkan), display interval about
6.05 ms (about 165 Hz). Release build of `main` at `6fc90b78`; the `automation`
feature only where a journey or the stress run needed it. Measurements are
single runs unless noted; raw outputs are kept outside the repository.

| What | Result | How |
|---|---|---|
| Build | Release build of the Studio with the `automation` feature: 3 min 49 s with dependencies already built; plain release rebuild of the Studio crate: 1 min 42 s; executable 21.6 MB | `cargo build --release -p agq-studio-native [--features automation]` with lean settings (no incremental build, no debug info, four jobs) |
| Start | Main window exists after about 30–80 ms (about 450 ms on the first start after a build). Start to two frames and exit: 0.65–0.85 s warm, 1.44 s first start. The first frame itself is not timed | PowerShell launch with `--frames 2`, polling the process every 10 ms |
| Project open | In process: 2.56 ms (129 ms the first time, before an identity file exists). The Studio's start with `--project` was indistinguishable from the start screen (0.68–0.84 s to two frames and exit) | `cargo run --release -p agq-system-state --example project_timing`; launch with `--project` |
| Scene build (every change rebuilds it) | URL shortener projects: 0.09–0.11 ms (6–12 cards); architecture fixture 0.26 ms (28 cards); 1k: 114 ms (layout 111 ms); 10k: 2,662 ms (layout 2,580 ms, index 82 ms) | `--frames N --metrics` (`scene_build_ms`) |
| Edit to Surface (derived) | Scenario A size about 11 ms (5 ms apply and save, 0.1 ms scene, one frame); 1k about 120 ms; 10k about 2.7 s | Sum of measured parts; not measured end to end |
| Steady frames | 1k: interval median 6.06 ms, p95 6.42 ms; UI CPU p95 1.47 ms; GPU pass p95 0.15 ms. 10k: median 6.03 ms, p95 8.47 ms; UI CPU p95 7.14 ms; GPU pass p95 1.43 ms; 8,788 of 10,200 cards visible | `--fixture stress1000/stress10000 --frames 600 --gpu-timestamps --metrics` |
| Pan and zoom | 1k: pan p95 6.37 ms, zoom p95 6.21 ms, input to next update p95 5.7 ms, UI CPU p95 1.90 ms. 10k: **pan median 24.0 ms, p95 34.1 ms; zoom median 16.4 ms, p95 31.7 ms**; input to next update p95 32.1 ms (pan), 28.9 ms (zoom); UI CPU median 11.5 ms, p95 21.9 ms; GPU pass p95 2.1 ms | `--scenario stress --gpu-timestamps --scenario-report` (120 pan and 120 zoom intervals after warm-up) |
| Memory | Start screen and URL shortener projects: 322–325 MB peak working set, 351–353 MB private. `a-build` journey: 402 MB working set. 1k: 342–346 MB. 10k: 402–414 MB working set, 508–525 MB private | Process counters polled every 10 ms. What makes up the 322 MB was not investigated (R-44) |
| Journeys | `a-build` passed (74 steps, 5.7 s), `a-crash` passed (5), `a-reopen` passed (15), `a-assistant` passed (19, 3.6 s) | Commands in `docs/stages.md` |

### 5.3 Gaps against this phase

| Gap | Where | Consequence | Stage |
|---|---|---|---|
| The Assistant has never run against a live model | `docs/stages.md` Stage 3 | Whether it follows the skills is unknown (A-4) | 4 |
| Thinking text is empty | `claude.rs` sends `thinking` without `display`; `claude-opus-5` defaults to omitted [11] | No visible thinking | 4 |
| The Conversation showed a `sysml` code block | `a-assistant` screenshot (scripted text); no skill forbids it | Conflicts with C-4 | 4 |
| `read_model` returns the whole model | `crates/assistant/src/tools.rs` | Context grows with the model | 4 |
| No live evaluations | — | Model and skill changes are judged by feel | 4 |
| Q-6 decided on thin evidence | The Stage 2 prototype measured chat frame time only; selection, typography, motion and screen readers were not tested | The AAA bar may be capped by the toolkit | 4 (S4.1) |
| egui is three releases behind | 0.33.3 in use; 0.36.2 released 2026-09-08; hinting, variable-font weights, shaping and IME work arrived in 0.34–0.35 [70][73] | Known text gaps may already be fixed | 4–5 |
| The 10k Surface misses its budget | §5.2: pan p95 34.1 ms; CPU-bound | Scenario D7 fails | 5 |
| Every change rebuilds the whole scene | `StudioApp::rebuild` in `crates/studio-native/src/app.rs` | 2.7 s per edit at 10k | 5 |
| No icons; four fixed columns; the Surface gets about 42% of a 1600 px window; cards are mostly empty below the title; every new card glows at once; problem text is long prose; the lock prompt is a plain dialog | screenshots of the journeys | Below the C-32 bar | 5 |
| The 10k overview is an unreadable mass of lines | `--fixture stress10000` screenshot | Scenario B readability | 5 (levels of detail), 9 (overviews) |
| No Settings view; keys only in environment variables | — | Scenario E; A1's message cannot link to a fix | 5 |
| One provider, hand-written client | `crates/assistant/src/claude.rs` | C-34, C-35 | 5 |
| Conversation stored as raw Anthropic content blocks | `crates/assistant/src/conversation.rs` | Not provider-neutral (R-23) | 5 |
| The Operator cannot type while a turn runs | `crates/studio-native/src/conversation.rs` refuses sending | No steering (C-39) | 6 |
| No plan, notes, skill files, compaction or autonomy modes | — | Scenario F | 6 |
| No agents in the subset | `docs/subset.md` | Scenario G | 6–8 |
| No performance checks in CI | `.github/workflows/ci.yml` | Regressions go unnoticed | 4 |
| Memory about 320 MB at rest, not understood | §5.2 | Budget unknown until investigated (R-44) | 5 |

### 5.4 Keep, simplify, replace

| Area | Disposition | Reason |
|---|---|---|
| `crates/language` | **Keep** (locked core); in Stage 6 add `enum def` and the `Agents` library with recorded decisions | C-23, C-42, R-16 |
| `crates/system-state` | **Keep** (locked operations); previews reuse "try on a copy" and the "what changed" comparison | C-25, R-29 |
| `crates/history` | **Keep** (locked format) | C-24 |
| `crates/assistant`: turn loop, tools, conversation, tests | **Keep and extend** (plan, notes, skills, steering, compaction, autonomy modes) | R-21 |
| `crates/assistant/src/claude.rs` | **Replace** by `agq-providers` on rig after S4.2 | C-34 |
| Compiled-in skills | **Simplify** into `SKILL.md` files, still compiled in as defaults | C-41 |
| Conversation format | **Replace** with format 2 (provider-neutral); format 1 conversations are imported read-only as transcripts | R-23 |
| `crates/studio-native` | **Keep and restructure**: tokens and components, docked panels, Settings view, Conversation polish; journeys extended | C-32 |
| `theme.rs` | **Replace** by the tokens module and generated themes | R-26 |
| `markdown.rs` | **Keep and extend** (selection, tables), unless S4.1 decides otherwise | S4.1 |
| `gpu.rs`, `scene.wgsl` | **Keep** | Not the bottleneck; carries over to any toolkit that can host it (S4.1) |
| `crates/studio-scene` | **Keep**; add incremental layout and routing, named level-of-detail tiers | R-28 |
| Session file | **Simplify**: presentation state only; preferences move to `settings.json` | R-43 |
| Automation journeys and stress run | **Keep** (feature `automation`); add budget assertions and journeys D, E, F | R-27 |
| `tools/check_architecture.py` | **Extend**: network and async crates only in Providers; tolerate new model constructs | R-41 |
| CI workflow | **Extend** with CPU-side budget checks | R-27 |
| Standards pinning tools and `standards/` | **Keep** | Unchanged (§8.1 rule 8) |
| `REALIGNMENT.md` | **Retire** to git history in W4.1 | C-47 |

### 5.5 Concepts carried forward

- **Stable element identity.** Identity is separate from names and paths;
  rename and move preserve it. Ambiguous text edits need reconciliation, not
  silent reassignment.
- **Atomic change against a known base state.** A malformed or stale change is
  rejected and leaves the state unchanged; a well-formed change that makes the
  model invalid is applied and shown at the elements concerned (C-25).
- **Unsupported semantics fail loudly.** They are never approximated.
- **Simulation disciplines** (Stage 7). Deterministic, normalised traces. A
  *run completing* is separate from a *check passing*. Runs are pinned to the
  model state they started from and isolated from each other and from the
  design. Explicit stop reasons (`condition_met`, `input_exhausted`,
  `ambiguous_transition`, `missing-recording`, limits). No external side effects
  during simulation; live evaluations of agents run apart (C-43).
- **Durability practices.** Write atomically, never acknowledge before it is
  durable, refuse an unknown format version. They apply to every new file in
  §4.9.
- **Provider isolation.** AI transport never enters the language core or the
  System State, and a failed provider never blocks manual work.

---

## 6. Sequence of stages

Each stage ends with something the Operator **uses** (C-15). Automated tests
support each stage but are never its acceptance on their own. **No work outside
the current stage**: items listed under "Waits" do not start early.

Stage numbers continue `REALIGNMENT.md`'s. The stages it called 4 (simulation)
and 5 (implementation linking) are now Stages 7 and 8; its later Stages 6
(Scenario B) and 7 (Scenario C) are now 9 and 10.

**Before fanning out builders** in any stage, the interfaces listed for it are
written, reviewed and merged first; builders then work in parallel against them
on separate branches, each small enough to review.

**Build constraints on the reference machine.** It has little free disk (about
20 GB). Builds use lean settings (no incremental build, no debug info, three or
four jobs), parallel worktrees share one target folder, and spikes run one at a
time.

### 6.1 Stages 0–3: status

Built and recorded in `docs/stages.md`; all **provisionally complete, pending
the Operator's acceptance**, which happens live at the start of Stage 4 (C-29).
Their overnight decisions are confirmed (C-23 to C-27, C-30), except Q-6, which
is reopened (C-28).

### 6.2 Stage 4: live proof and foundations

**Outcome.** The Operator runs Scenario A live (§2.2) and accepts or rejects
Stages 0–3. The phase's two spikes are decided. The interfaces that Stages 5
and 6 build on are merged. Performance budgets are measured continuously.

**Work items**

- **W4.1 Retire `REALIGNMENT.md`** (first item, one pull request). Remove it;
  point `AGENTS.md`, `README.md`, `docs/stages.md`, `models/agentique/README.md`,
  `models/agentique/Agentique.sysml`, `tools/check_architecture.py`, source
  comments and the crate READMEs at this document; update AGENTS.md's rules to
  §8.1; renumber stage references (simulation is now Stage 7, implementation
  links Stage 8) in `docs/subset.md`, `docs/stages.md` and the self-model. A
  search for "REALIGNMENT", "Stage 4" and "Stage 5" finds every place.
- **W4.2 Fixes that make live acceptance meaningful** (they serve Stage 3's
  scenario steps, not new features): request thinking summaries (R-31); a skill
  rule and an evaluation check that the Assistant never shows SysML text (C-4);
  the `read_model` outline and tool-result caps (R-34).
- **W4.3 The evaluation set** (R-19): 20–30 Scenario A tasks with scripted
  Operator answers, graded on the System State, three trials each, run on
  demand with a key.
- **W4.4 Live acceptance** (§2.2) by the Operator; the result is recorded in
  `docs/stages.md`. Failures become work items here, before anything builds on
  them.
- **W4.5 Spike S4.1: toolkit** (Q-6, C-28). See below.
- **W4.6 Spike S4.2: rig parity** (C-34). See below.
- **W4.7 Continuous budgets** (R-27): a start timestamp in the metrics report;
  budget assertions in the stress harness; CPU-side ceilings in CI; a
  reference-run command documented in `docs/stages.md`.
- **W4.8 Self-model**: add the planned `Providers` part and its dependencies;
  extend the check (§4.6).

**Spike S4.1: toolkit** (R-20). Ten working days, hard stop, throwaway
branches only; nothing merges. Exception (§7.6, 2026-09-27): Track A's code may
be kept and merged as W5.1 after review.

- **Tracks.** (A) egui upgraded to 0.36, plus the four "to the bar" items:
  selection across Conversation messages, real Inter weights from the variable
  font, a spring and tween motion layer, and screen-reader names for the Surface
  and chat. (B) GPUI with GPUI Kit, pinned to exact snapshot versions: the same
  chat fixture, an Inspector-like panel, and the Surface drawn with GPUI's own
  drawing at 10k elements. Track B stops at the end of its second day if the
  Surface is over twice the frame budget or the Windows build is broken.
- **Why these two.** Since 0.33, egui gained hinting, weights from variable
  fonts, shaping and an IME rewrite (0.34–0.35); at 0.36.2 (2026-09-08) it still
  lacks selection that
  survives scrolling, subpixel text, inline widgets in wrapped text, springs and
  live regions [70][71][72][73]. GPUI is the only option whose ceiling on text,
  motion and components is clearly above egui's, but on Windows upstream it
  cannot show our wgpu renderer's output (a Direct3D 11 renderer; surface
  painting is macOS-only [75]; only the community fork has a wgpu API [76]), the
  official crate has been frozen at 0.2.2 since 2025-10-22, GPUI Kit depends on
  weekly third-party snapshots [74][79], a comment attributed to a Zed engineer and
  quoted second-hand on Hacker News said GPUI work would get "some major
  brakes" [77], and screen-reader support is experimental
  with the Windows issue open [78]. Slint is the reserve (it can draw our wgpu
  Surface beneath its UI [80], but its rich text and selection do not beat our
  own chat renderer). Iced (no upstream accessibility [81]), Xilem, Freya, Blitz and
  Makepad are not ready for this bar; a web UI in Tauri conflicts with C-9.
- **Fixtures.** A 10k-element model; a 200-message Conversation (about 60,000
  words, 50 code blocks, 100 tool cards, 300 element links) streamed at 100
  tokens per second; a 2560×1440 window at 150% scaling, repeated at 100%.
- **Hard gates** (a track must pass all): G1 the toolkit's own cost, with the
  same Surface code in both tracks: CPU time of camera-only frames ≤ 2 ms at
  10k, and frame time p95 ≤ 8.3 ms (max ≤ 16.7 ms) over 30 s of pan and zoom
  at 1k with all panels open and chat streaming (reaching C-33 at 10k is
  Stage 5 work in our own code, W5.5, whichever toolkit wins); G2
  input-to-present p95 ≤ 12 ms; G3 chat scroll and streaming p95 ≤ 8.3 ms,
  reopen ≤ 150 ms; G4 drag selection across three or more messages
  that survives auto-scroll and copies correctly; G5 real weights 400, 500 and
  600 at 100%, 150% and 200%; G6 at least 18 of 20 checklist items read
  correctly by Narrator; G7 Japanese IME composition at the caret; G8 clean and
  incremental builds from crates.io within this machine's disk limits.
- **Then** the Operator scores text, motion, "does not look like a widget
  toolkit", and risk, blind where possible.
- **Decision rule, set before the spike.** Switch to GPUI only if it passes every
  gate, **and** egui fails G4, G5 or G6 with no credible fix within two weeks or
  the Operator scores GPUI at least one point higher on both text and motion,
  **and** the estimated migration is ten weeks or less, **and** the Operator
  explicitly accepts the governance risk. Stay with egui 0.36 if it passes every
  gate and the Operator rates it at the bar, or if GPUI fails any gate. If both
  tracks fail a hard gate, the choice comes back to the Operator (Parley inside egui for
  the Conversation, Slint with custom chat blocks, or reopening C-9 for a web
  Conversation).
- **Honest prior** *(not a decision)*: about 60% stay with egui, about 30%
  GPUI wins on feel, about 10% neither is clearly adequate. Estimated work to
  the bar: egui 5–7 weeks (not the "about two weeks" recorded in Stage 2); a
  GPUI migration 8–12 weeks plus ongoing churn.

**Spike S4.2: rig parity** *(provisional parts of R-21, R-22)*. At most five
working days on a throwaway branch: an `agq-providers` prototype on rig with
the Stage 3 loop unchanged.

- **P1** The loop and tool tests (`tests/turn.rs`, `tests/tools.rs`), the
  Studio's conversation tests and the `a-assistant` journey pass against the new
  provider layer (the scripted model is unchanged); the client tests
  (`tests/claude.rs`) are replaced by equivalent tests of `agq-providers`
  against recorded streams.
- **P2** Live on Anthropic: streamed text; thinking summaries; effort; tool
  input streamed as generated; prompt cache reads on the second call; usage with
  cache counts; stop within 50 ms; the retry rule; a refused reply runs no
  tools; and on `claude-opus-5-5` the re-serialised history is accepted. Newer
  Claude models tie thinking blocks to their model and conversation and refuse
  edited history for newer accounts (Anthropic's migration guidance; not
  re-checked against the live page).
- **P3** Live on OpenAI and OpenRouter: five evaluation tasks end to end with
  tools, streaming, and reasoning summaries where offered.
- **P3a** Live on DeepSeek (`deepseek-flash`, effort `high`, C-35): five
  evaluation tasks end to end with tools, streaming, and `reasoning_content`
  sent back across tool turns.
- **Without a provider's key**, its live items run against canned streams and
  the local HTTP stand-in and are marked "not tried live" (§7.6).
- **P4** Build: clean and incremental builds within the disk limits; the TLS
  option chosen (§4.7); incremental build time of the Studio at most 30% above
  today's.
- **P5** No rig type crosses `agq-providers`' public API (checked by review of
  its public items; the architecture check ensures no other crate depends on
  rig).
- **P6** Server-side fallbacks (Q-18): measure the behaviour and choose between
  an upstream contribution to rig (preferred if its maintainers accept it within
  the stage), a thin adapter in `agq-providers`, or no server-side fallbacks with
  "Retry with another model" (acceptable if no refusal appears in the evaluation
  set). Dropping server-side fallbacks changes C-27 and needs the Operator's
  decision.
- **Decision rule.** Proceed with rig for every provider (C-34) if P1–P5 pass,
  with live items that could not be tried marked so. If a P2 item fails with no
  workaround, bring the specific loss to the Operator instead of reverting by
  default.
- **Kept code (exception, §7.6, 2026-09-27).** Because C-35 requires the
  product to work end to end with only a DeepSeek key during Stage 4's live
  acceptance, the S4.2 provider layer is reviewed and merged as the start of
  W5.7 instead of being thrown away: `agq-providers` with the DeepSeek path
  used by the Assistant, and the other providers' paths tested without keys.

**Interfaces to define and merge before Stage 5 fans out**

1. `agq-providers` public API (§4.7): ids, model info and capabilities, request,
   stream events, reply, usage, errors, key status; the capability table.
2. The settings table (id, label, description, default, scope, allowed values,
   validation message) and `settings.json` format 1.
3. The tokens module and the component list (names and states), with the
   gallery fixture.
4. Conversation format 2 with the entries Stage 5 needs (Operator message,
   Assistant reply, tool results, notice), and entry kinds that later stages
   add without a new format.
5. The Assistant's provider-neutral event protocol (text, thinking summaries,
   tool input, usage).
6. The budget report fields shared by the harness, CI and the reference run.

**Acceptance evidence**

- The Operator has run §2.2 live and accepted Stages 0–3, or listed what fails
  and seen it fixed.
- S4.1 and S4.2 are decided by their rules, and the decisions are recorded in
  §7.6.
- The evaluation set exists; its first results are known to the Operator (not
  committed).
- CI runs the CPU-side budget checks; the reference run reports the §3.3
  baselines.

**Depends on:** adoption of this document. **Waits:** Studio polish, Settings,
new Assistant features, agents.

### 6.3 Stage 5: the daily-use Studio and Settings

**Outcome.** Scenarios D and E work at the C-32 bar on the reference machine.
The Surface meets its budgets at 1k and 10k (C-33). The Assistant runs through
the provider layer on rig, with Anthropic, OpenAI, OpenRouter and DeepSeek
selectable (C-34, C-35), and behaves as in Stage 3.

**Work items**

- **W5.1 Toolkit.** Apply S4.1's decision: upgrade egui to 0.36 and do the four
  "to the bar" items, or migrate.
- **W5.2 Design system.** Tokens and generated themes; components with every
  state in the gallery; one icon set after a licence check; typography decided
  side by side (Q-17); the literal-value check of §8.5 rule 1.
- **W5.3 Shell.** Surface-first layout; docked, resizable, collapsible panels;
  focus mode; status bar; layout remembered per project.
- **W5.4 Surface.** Dot grid; new cards (kind, name, type, badges; sized to
  content); ports by direction shape, hollow or filled; edges with arrowheads
  by kind and label pills that never overlap containers; selection ring outside
  the card; change marks by actor; named level-of-detail tiers; minimap
  (should); zoom controls; the standard zoom shortcuts; "go to element".
- **W5.5 Performance.** Incremental layout and routing (R-28); cached label
  layout; CPU culling at 10k; memory investigation (R-44); budgets enforced
  (§3.3).
- **W5.6 Conversation.** Selection across messages, virtualised list, tables,
  message actions, new tool cards, composer with context chips and model picker
  (§3.6).
- **W5.7 Providers.** `agq-providers` on rig for Anthropic, OpenAI,
  OpenRouter and DeepSeek, and the thin Jev client, whose key Settings tests in
  Scenario E (E4) before Stage 7 uses it (R-21, R-22, C-35); the Assistant moved onto it; conversation format 2,
  with existing conversations moved into the per-project folder (R-23, R-43);
  the self-model maps the crate (§4.6).
- **W5.8 Settings.** The view, the table, search, deep links, `settings.json`,
  keys in the Credential Manager, model lists and capability badges, cost
  display per turn and per day (§3.7, R-24, R-25, R-42).
- **W5.9 First run and empty states** (R-46).
- **W5.10 Accessibility** (§3.5).
- **W5.11 Quality-of-life "must" items** (§3.4).
- **W5.12 Journeys** `d-daily` and `e-settings`; existing journeys updated.

**Spike S5.1: incremental layout** (two days, before W5.5 fans out). Decide how a
change maps to the containers and edges it affects, keeping unrelated cards
still (`LayoutMemory` already keeps positions). Decision criterion: an edit on
the 10k fixture reaches the Surface in 100 ms or less without moving unrelated
cards.

**Interfaces to merge first:** the panel docking API; card, port and edge
drawing parameters from tokens; the settings table's first rows; the
`agq-providers` crate skeleton.

**Acceptance evidence**

- The Operator uses the Studio for real architecture work over at least a week
  and judges the daily paths at the bar in a side-by-side review (§3.1).
- The reference run meets every §3.3 budget marked for this stage; CI checks
  are green.
- The Operator performs Scenario E with real keys for every supported
  provider.
- Journeys `a-build`, `a-crash`, `a-reopen`, `a-assistant`, `d-daily` and
  `e-settings` pass.

**Depends on:** Stage 4. **Waits:** plan, steering, notes, skill files,
compaction, autonomy modes, agents.

### 6.4 Stage 6: the agentic Assistant and agents in the model

**Outcome.** Scenario F works on at least two providers, and Scenario G steps
G1–G3: the Operator designs agents, and sees Agentique's own Assistant as an
agent in its self-model.

**Work items**

- **W6.1 Autonomy modes** (C-38, R-29), with previews on the Surface for "Ask
  before every change" and for major-decision questions.
- **W6.2 Plan card** with `update_plan` (R-30).
- **W6.3 Visible thinking** per provider, with the degradations of §4.8 (R-31).
- **W6.4 Steering:** queue, Stop and send, take back; Surface edits reported to
  the Assistant (R-32).
- **W6.5 Compaction** (R-33).
- **W6.6 Skill files** with `read_skill`; Settings › Assistant › Skills (R-36).
- **W6.7 Notes** with `propose_note`; Settings › Assistant › Notes (R-35).
- **W6.8 Turn summary and review:** changes grouped per turn, problems before and
  after, cost; per-change Keep or Undo and Follow mode (should).
- **W6.9 Agents in the language:** `enum def` and enumeration values in the subset, the built-in
  `Agents` library, the validity rules, deviation 12, the subset manifest
  updated (both locked-core changes recorded in §7.6 first, R-16, R-37).
- **W6.10 Agents in the Studio and the self-model:** the agent badge and the
  Inspector section (R-38); a `designing-agents` skill; the Assistant modelled
  as an agent; `dependency` added to the subset and the self-model validated
  in CI (R-41).
- **W6.11 Evaluation set extended:** long tasks, the three modes, steering,
  designing agents; run on each provider to confirm the capability table (A-8).
- **W6.12 Journey** `f-long-task`.

**Spike S6.1: agent modelling** (three days, before W6.9). Model three agents:
link screening (fast, with a blocklist fallback), support triage (fast, with a
deliberate agent for hard cases), and Agentique's Assistant (deliberate). Judge
with the Operator how they read on the Surface and in the Inspector. Decide the
validity rules and whether escalation needs anything beyond two agents and a
routing part (Q-13).

**Interfaces to merge first:** schemas of `update_plan`, `read_skill`,
`propose_note` and `ask_operator.preview`; the skill loader's contract; the
notes store; the autonomy mode and its effect on the executor; the `Agents`
library text and validity codes; the event protocol and conversation entries
for plans, queued and delivered messages, previews, note proposals and
compaction summaries.

**Acceptance evidence**

- The Operator runs Scenario F with a task of at least about twenty tool calls,
  in each autonomy mode, on at least two providers: steers, stops and sends,
  refuses a lock change, undoes a turn, accepts and rejects notes, adds a skill.
- The Operator runs G1–G3.
- Must-hold behaviours pass in every trial of the evaluation set: never claims
  an unconfirmed change; never changes a lock without confirmation; asks on major
  decisions in the default mode; never shows SysML text.

**Depends on:** Stage 5. **Waits:** simulation and implementation of agents; the
Orchestrator.

### 6.5 Stage 7: simulation, with agents

**Outcome.** Scenario A step A5 and Scenario G steps G4–G6. Scenarios built from
Agentique's own parts run over the architecture and report whether it can carry
them out and whether requirements hold (C-16); agents run as stubs or
recordings, and live evaluations run apart (C-43).

**Work items**

- **W7.1 Spike S7.1: simulation semantics** (Q-5): what "simulate a contract"
  means for message and interface flows, and the minimal state and action
  semantics needed. Decision criteria: the URL shortener's three main scenarios
  and the link-screening scenario can be written, run and explained on the
  Surface, and a broken interface is caught. The subset extension is recorded in
  the manifest. The spike also settles how a requirement states an agent's
  required pass rate (Q-14).
- **W7.2** Scenarios, runs and traces with the disciplines of §5.5; step-through
  playback on the Surface (after IcePanel's flows [59]).
- **W7.3** Agent stand-ins with injected failures; recordings keyed by request
  digest; `missing-recording` as a stop reason (R-39).
- **W7.4** Live evaluation runs of agents: samples, pass rates, cost; keeping a
  good run as recordings (R-39).
- **W7.5** The `Simulation` crate and its place in the self-model (Q-20).

**Interfaces to merge first:** the scenario format, the trace format, the
recording format and location (Q-16), the evaluation report (never committed).

**Acceptance evidence.** The Operator simulates the URL shortener's main
scenarios and sees a deliberately broken interface caught before any code
exists, then fixes it. The Operator simulates link screening with injected
failures and sees the fallback path, then runs a live evaluation and keeps a
run as recordings.

**Depends on:** Stage 6.

### 6.6 Stage 8: implementation links, with agents

**Outcome.** Scenario A steps A6–A7 (**Scenario A complete**, C-19) and
Scenario G steps G7–G9.

**Work items**

- **W8.1 Spike S8.1: checks** (Q-2): which model-to-code checks give real
  protection (interface signatures, dependency direction, required tests, test
  results), for ordinary parts and for agents (C-46). Decision criterion: each
  kept check catches a deliberate contradiction in the URL shortener with no
  false alarm on the correct code.
- **W8.2** The Assistant's code tools, and how the autonomy modes extend to side
  effects outside the System State (Q-15).
- **W8.3** Implementation links, drift and per-part status on the Surface; model
  and code sharing commits (C-24).
- **W8.4** Agent checks: ports against code types, fallback path, guardrail
  tests on recordings, live pass rate against the requirement (R-40).

**Acceptance evidence.** **The Operator runs the complete Scenario A**,
including trying the generated system and the "expiring links" change with a
locked part refusing silent change; implements link screening and sees a
deliberate drift.

**Depends on:** Stage 7.

### 6.7 Later stages (order to be confirmed when reached)

- **Stage 9. Scenario B (larger system):** readable overviews at scale,
  grouping, branches in the UI, merging by identity, keeping concepts general.
- **Stage 10. Scenario C (dogfood):** read an existing codebase into a model
  (Q-7); Agentique opens and changes itself.
- **Afterwards:** the Orchestrator and assistant roles (C-5, Q-4); two-way
  reconciliation (C-7); links to deployed systems; local models, Gemini and
  typed fast-decision APIs other than Jev (Q-12); spending limits if wanted.

---

## 7. Decisions and open questions

### 7.1 Confirmed Operator decisions

C-1 to C-22 are carried from `REALIGNMENT.md` unchanged, except C-6, C-9,
C-14 and C-22, which are edited in place (§7.6).

| ID | Decision |
|---|---|
| C-1 | Agentique is where a project's *development* lives for its whole lifecycle; the project itself (code, deployment) can live anywhere. The primary focus is software systems |
| C-2 | One System State is the single truth. The Operator changes it on the Surface and the Assistant through tools. All changes appear live |
| C-3 | The Studio has a visual Surface (with side panels) and a Conversation as equal ways to work |
| C-4 | KerML/SysML are the foundation, not scripture. Use a deliberate subset, grown by need; completeness only where it pays off. Interchange with other tools is not required. Users never see SysML text. Chosen for their accumulated research, to avoid an AI-invented home-grown model |
| C-5 | One Assistant with tools and skills first; it becomes an Orchestrator of assistants later |
| C-6 | The Assistant acts where it is confident and asks the Operator about major decisions. Refined by C-38: this is the default autonomy mode; in "Ask only on locks" the Operator waives these questions for a conversation |
| C-7 | Model to code: "the model is the contract; code linked and checked" is the working direction, two-way reconciliation is the long-term goal. Both are provisional pending experiments. Linking to deployed systems comes later |
| C-8 | All Assistant actions are visible in real time |
| C-9 | Single user (the Operator). A native Rust application on Windows. Several model providers through rig (C-34, C-35), replacing "the Claude API first". The Operator delegated the history/version-control decision (now C-24) |
| C-10 | The definition of slop and non-slop in §1.3, including "stable core, experimental edges" |
| C-11 | Parts can be locked. Changing a locked part requires the Operator's confirmation. Protecting established parts against idea-driven drift is essential |
| C-12 | Architecture-first thinking and first-class visual architecture ("like Unreal Engine") are the main cure for slop |
| C-13 | Agentique will be dogfooded to fix this repository |
| C-14 | The non-goals in §1.6: the first seven rows are the original C-14; the rows after them follow from the decisions named in each row |
| C-15 | Evidence means the Operator using Agentique and systems built with it, not verification records |
| C-16 | Simulation is testing of the architecture/contract, like tests for code, built from Agentique's own parts |
| C-17 | Build order: architecture, then simulation, then implementation. Use is a continuous loop, not a pipeline |
| C-18 | Proof order: a small new system, then a larger system, then Agentique itself |
| C-19 | Scenario A (URL shortener journey) is the first proof. Stage split: architecture first, implementation second |
| C-20 | Agentique's own architecture follows KerML/SysML principles, so it enjoys the same benefits it promises |
| C-21 | Very high quality in visuals, features and UX. A modern developer- and AI-inspired aesthetic. A chat panel on par with modern AI chat interfaces |
| C-22 | "Retire" means preserved at a git tag or in git history, then removed from the active repository |

Confirmed in the interview of 2026-09-27:

| ID | Decision |
|---|---|
| C-23 | R-3 confirmed: the language core is the rebuilt `agq-language`; Generation 2 stays retired |
| C-24 | R-6 confirmed as built: git through `git2`; `model/` with `*.sysml` and `agentique.json` in the format of §4.5; continuous saving that writes only changed documents; commits at checkpoints; a project's repository only if rooted at the project folder; merges by element identity, never git's text merge; deleted targets unbind by name as after a reload; a checkpoint is refused while model files have been edited outside the app |
| C-25 | R-18 confirmed: a malformed change is rejected and leaves the state unchanged; a well-formed change that makes the model invalid is applied, shown at the elements and can be fixed or undone |
| C-26 | Q-9 confirmed: the conversation is stored per project in the app's local data, not in the model folder or the code repository |
| C-27 | The Assistant's Stage 3 defaults are confirmed as starting defaults, now exposed in Settings: `claude-opus-5`, effort `high`, server-side refusal fallbacks where available, no tool to lock or unlock, and "Undo the Assistant's changes" undoing every change since the turn started |
| C-28 | Q-6 is reopened: the toolkit is decided by a spike against the higher bar (S4.1), not by the Stage 2 assessment |
| C-29 | Stages 0–3 are accepted only after the Operator runs Scenario A live at the start of Stage 4 (§2.2) |
| C-30 | Deviations 7–11 (stricter than the standard) are kept |
| C-31 | Stage order for this phase: foundations (4), daily-use Studio and Settings (5), agentic Assistant with agents in the model (6), simulation with agents (7), implementation links with agents (8). C-17's order is kept |
| C-32 | Ambition: Linear- and Figma-grade on every path used daily; rarely used paths clean but plain. Done means the Operator prefers Agentique to a whiteboard plus an IDE for architecture work |
| C-33 | Surface performance: p95 frame time ≤ 8.3 ms at 1k elements and ≤ 16.7 ms at 10k during pan and zoom; an edit on a 10k model reaches the Surface in ≤ 100 ms |
| C-34 | Provider independence through rig for every provider, Claude included, using escape hatches where rig lacks a feature. Clarified 2026-09-27: a provider released rig does not support (TypeSafe AI's Jev, a typed decision API) gets a thin client inside `agq-providers`, behind the same boundary, until rig releases it |
| C-35 | Providers in this phase: Anthropic, OpenAI, OpenRouter and DeepSeek (model `deepseek-flash`, effort `high`), and TypeSafe AI's Jev as a model provider for fast ("system 1") agents in designed systems, never for the Assistant. Amended 2026-09-27: the product works end to end with only a DeepSeek key configured |
| C-36 | API keys are stored in the Windows Credential Manager |
| C-37 | Costs are shown; there are no spending limits |
| C-38 | Three autonomy modes, after Claude Code and Codex: ask before every change; act, but ask on major decisions; act freely, asking only on locks |
| C-39 | Long tasks show a visible plan, and the Operator can steer (queue messages, stop and send, stop, undo the turn) |
| C-40 | Memory: short notes the Assistant proposes and the Operator approves, per project and app-wide; nothing is remembered silently |
| C-41 | Skills: built-in skills plus the Operator's own skill files, app-wide or per project |
| C-42 | The concept of AI-driven parts is named **agent**. An agent is modelled as a part definition that specialises a built-in library definition (`Agents::Agent`) |
| C-43 | Agents in simulation run as deterministic stand-ins or recorded answers; live calls are separate evaluation runs |
| C-44 | Agentique's own Assistant is modelled in `models/agentique/` as the first agent |
| C-45 | Agents first appear in the Studio with the agentic Assistant (Stage 6) |
| C-46 | An agent in a designed system is implemented in the project's own code, linked and checked (provisional until Q-2) |
| C-47 | `ROADMAP.md` replaces `REALIGNMENT.md` as the single governing text; `REALIGNMENT.md` is retired to git history |

### 7.2 Recommendations

**Carried from `REALIGNMENT.md`, with their status**

| ID | Recommendation | Status |
|---|---|---|
| R-1 | Consolidate onto one line: Native Studio shell plus a simplified language core; retire Generation 1 and the browser/HTTP paths | Done (Stages 0–2) |
| R-2 | Remove the certification machinery from the working path | Done (Stages 0–2) |
| R-3 | Decide "extract" or "rebuild" of the language core by a spike | Decided: rebuild; confirmed (C-23) |
| R-4 | Load only the standard libraries the subset needs, built in, without authentication | In force: `ScalarValues`; `Agents` added in Stage 6 (deviation 12) |
| R-6 | Git history with SysML text and an identity and lock file | Confirmed (C-24) |
| R-7 | Merge `studio-native` into the root workspace; keep test harnesses out of the production binary | Done (Stage 0) |
| R-8 | One subset manifest | Done (`docs/subset.md`); grows in Stages 6–7 |
| R-9 | One deviations list, no profile versions | Done (`docs/deviations.md`); deviation 12 in Stage 6 |
| R-10 | Retire conformance registers and verification records; stage summaries instead | Done (Stage 0) |
| R-11 | A lock covers the part and what it owns | In force |
| R-12 | Stop and undo of Assistant work | In force; extended by steering (C-39) |
| R-13 | Plain names instead of coined ones | In force (§8.4) |
| R-14 | The provisional logical architecture | Superseded by the self-model (§4.6) |
| R-15 | An automated check that crate dependencies agree with the self-model | In force; extended (R-41) |
| R-16 | The stable core (language core, System State operations, persistence format) is locked | In force |
| R-17 | Minimise the Node toolchain | Partly done (one dev dependency); rewriting the pinning tools waits until they are run again |
| R-18 | Two kinds of failure in the System State | Confirmed (C-25) |

(`REALIGNMENT.md` had no R-5.)

**New**

| ID | Recommendation | Basis |
|---|---|---|
| R-19 | An evaluation set of 20–30 Scenario A tasks (extended with F and G), graded on the System State by code plus one calibrated rubric grader, three trials each; must-hold behaviours pass^3, capabilities pass@3; run on demand with a key; task definitions committed, results never | Stage 3 never ran live; Anthropic's evaluation guidance [23]; Gemini CLI's behavioural evaluations [30] |
| R-20 | Spike S4.1 (toolkit) as specified in §6.2, with its gates and decision rule set in advance | C-28; §6.2 |
| R-21 | A new part **Providers** (crate `agq-providers`), the only crate that depends on rig, tokio, reqwest or the credential-store crates; rig pinned exactly and upgraded deliberately; no rig types in its public API; the Assistant's turn loop stays ours and calls rig's completion models; rig's `AgentRun` adopted only if it removes code without hiding policy *(provisional, S4.2)* | C-34; rig's churn [1][4]; rule 6 of §8.1 |
| R-22 | Anthropic features on rig: thinking and effort through `additional_params`, never together with rig's `output_schema`; tool input streaming through raw tool definitions; our own retry wrapper; stop reasons checked before tools run; output limits always explicit; fallbacks as decided by S4.2 *(provisional)* | §4.7 [2] |
| R-23 | Conversation format 2: provider-neutral entries with `"format": 2`; format 1 conversations imported read-only as transcripts | Thinking blocks are tied to their model; rig normalises messages [2] |
| R-24 | The Settings view as in §3.7: one settings table, search, deep links, instant apply, per-row reset, a test before saving a key, `settings.json` with only changed values | Settings research: VS Code, Zed, GitHub, Microsoft, Primer, Raycast, Jan [35][37][39][41][43][44][45] |
| R-25 | The key handling contract in §4.9 | Zed and GitHub CLI on Windows [38][40]; `keyring` threading note [46] |
| R-26 | The design system of §3.2 as starting values, generated themes, one icon set, and a component gallery | Design research [49]–[69] |
| R-27 | Budgets enforced continuously: CPU-side ceilings in CI; frame, latency, start and memory budgets asserted by the harness on the reference machine before merging Studio or Surface changes and at stage ends; reports never committed | C-33; Zed, Warp and Figma treat speed as a checked property [61][55] |
| R-28 | Incremental Surface updates: a change re-lays out only the containers it affects and re-routes only the edges it touches; label layouts cached *(provisional, S5.1)* | §5.2: 2.7 s per edit at 10k |
| R-29 | The default autonomy mode is "Ask on major decisions"; the mode is chosen per conversation and sent as a short note with each Operator message, so changing it does not rebuild the prompt cache; previews reuse the "what changed" comparison | C-6, C-38; prompt caching [13]; keep the cached prefix stable [33] |
| R-30 | The plan card through an `update_plan` tool (3–7 short steps, one running), used for tasks of three or more steps and re-sent after compaction | C-39; Codex's `update_plan` [29]. Claude Code now leaves its task tools off by default on the newest models because they cost context [26], so the card is kept small and used only for longer tasks |
| R-31 | Thinking summaries (`display: "summarized"`) shown collapsed per step; progress updates as tool-card subtitles where offered; degradation per §4.8 | C-8; Claude thinking display [11] |
| R-32 | Steering: Enter queues a message delivered at the next tool boundary; "Stop and send"; a queued message can be taken back; Surface edits during a turn are named in the next tool result | C-39; Claude Code, Cursor, Zed [25][31][32] |
| R-33 | Client-side, provider-neutral compaction between turns at about 120,000 tokens or 60% of the window; a mid-turn pause at about 300,000 or 85%; Agentique's summary instructions; the full transcript kept for the Operator. Context editing (clearing old tool results) is not used [15] | Long conversations; rig cannot carry Anthropic's compaction blocks [2]; Codex and Gemini CLI thresholds [29][30]; on-demand compaction [14] |
| R-34 | Tool results capped at about 8,000 tokens with a hint to narrow the request; `read_model` without an element returns an outline | Anthropic's tool guidance [20] |
| R-35 | Notes through `propose_note` and an approval card; Markdown files per project and app-wide; sent after the skills, capped at about 2,000 tokens; architecture decisions go into the model, not notes | C-40; Gemini CLI, Devin [30][103] |
| R-36 | Skills as `SKILL.md` files in the Agent Skills format; core skills always loaded, others by description through `read_skill`; precedence project, app-wide, built-in | C-41; Agent Skills [21] |
| R-37 | The `Agents` library, the `enum def` and enumeration-value extension, the validity rules `wrong-fallback` and `agent-fallback`, and deviation 12, as in §4.11 *(provisional, S6.1)* | C-42; SysML 7.6.8 [82]; runtime assurance [85][86] |
| R-38 | Agents are marked on the Surface by shape and badge, not colour; the Inspector gets an Agent section | §3.2 principles |
| R-39 | Agent simulation modes as in §4.11: stubbed with injected failures, recorded with digest keys and `missing-recording`, live evaluations apart with pass rates | C-43; [88][89][90] |
| R-40 | Agent implementation checks: ports against code types, fallback path, guardrail tests on recordings, live pass rate against the requirement *(provisional until Q-2)* | C-46 |
| R-41 | Add the standard `dependency` relationship to the subset so that CI can validate `models/agentique/` with `agq-language`; `check_architecture.py` also forbids the rig crates, tokio, reqwest and the credential-store crates outside Providers (`reqwest` in `agq-assistant` is a temporary exception until W5.7) and tolerates model constructs it does not need | C-20; R-15 |
| R-42 | Cost display per turn and per day, from provider usage and a dated local price table marked as an estimate, per provider in Settings | C-37; the Models API has no prices [16] |
| R-43 | One per-project folder in the app's local data (`projects\<folder>-<hash>\`: conversation, notes, skills); existing conversations move there; the session file keeps only presentation state; preferences move to `settings.json` *(provisional for notes and skills, Q-10)* | C-26; one place per kind of data |
| R-44 | Investigate the memory footprint in Stage 5 (wgpu backend on Windows, font atlas, buffers) before fixing the memory budgets | §5.2: 322 MB at rest, not understood |
| R-45 | The evaluation set also compares default models and effort (for example `claude-opus-5` at `high` against `claude-opus-5-5` at `medium` and `high`) and every Assistant provider (Anthropic, OpenAI, OpenRouter, DeepSeek) before any default changes | Q-19; C-27 |
| R-46 | A three-step first run (what Agentique is; connect a provider or skip; create or open a project), with the URL shortener as a sample | Scenario E1 |

### 7.3 Assumptions

| ID | Assumption | Status or how it gets tested |
|---|---|---|
| A-1 | The Generation 2 core, without closure and audits, validates fast enough to feel live | Disproved in Stage 1 (72 s per edit); the core was rebuilt (C-23) |
| A-2 | A small KerML/SysML subset is enough for Scenario A | Holds for the architecture steps; simulation extends it (Stage 7) |
| A-3 | SysML text in git with an identity file preserves element identity well enough | Holds in the Stage 1–2 tests; unnamed elements are matched by position (a known limit) |
| A-4 | The Claude API with tool use produces coherent architecture changes through typed tools | Open: tested live in Stage 4 (§2.2, R-19) |
| A-5 | egui (with custom rendering) can reach the quality bar, including the Conversation | Open: S4.1 |
| A-6 | Nothing in Generation 1 is used by anyone else | Held; retired with no reported cost |
| A-7 | rig's churn can be contained in `agq-providers` and absorbed on our schedule | S4.2, then every rig upgrade |
| A-8 | OpenAI, OpenRouter and DeepSeek models use the Assistant's tools and follow its skills well enough for daily work | Evaluation set per provider (Stages 4 and 6); DeepSeek live first (C-35) |
| A-9 | Incremental layout brings a 10k edit to the Surface in ≤ 100 ms, and CPU culling and label caching bring 10k pan and zoom to ≤ 16.7 ms p95, without a toolkit change | S5.1, W5.5 |
| A-10 | An agent's contract fits the current subset plus `enum def` and enumeration values (ports, requirements, a redefined `fallback`) | S6.1 |
| A-11 | Recorded agent answers stay valid long enough between prompt changes to be useful | Stage 7 |
| A-12 | Client-side compaction keeps decisions well enough over long conversations | Evaluation tasks with compaction (Stage 6) |

### 7.4 Open questions

| ID | Question | Blocking? |
|---|---|---|
| Q-1 | What "Agentic" reference the Operator mentioned as inspiration in the first interview | No |
| Q-2 | Which model-to-code checks are feasible and worthwhile, for parts and for agents | Blocks Stage 8 design (S8.1) |
| Q-3 | What exactly counts as a "major decision" | Partly resolved: the autonomy modes (C-38) settle the levels; the definition lives in the `decisions` skill and is refined with the evaluation set |
| Q-4 | Orchestrator design: which assistant roles, how they coordinate (multi-agent systems cost about 15 times the tokens of a chat [22]) | No (after Stage 8) |
| Q-5 | What simulation semantics are needed to test contracts (message flows, states, actions, time?) | Blocks Stage 7 design (S7.1) |
| Q-6 | Whether egui is the right toolkit for the quality bar | Reopened (C-28); decided by S4.1 |
| Q-7 | How to read an existing codebase into a model | No (Stage 10) |
| Q-8 | Which parts of the SysML standard the Operator would miss under the subset | No (reviewed as it grows) |
| Q-9 | Where conversation history is stored | Resolved (C-26) |
| Q-10 | Should per-project skills and notes live in the project folder, versioned with the code, instead of the app's local data? | Decided overnight 2026-09-27, pending Operator confirmation: the app's local data, as in R-43 |
| Q-11 | Should Jev or another typed fast-decision API become an agent's model provider, once rig releases `rig-typesafeai` [9]? | Resolved by the Operator (C-35): Jev is a model provider for fast agents now, through a thin client until rig releases it (C-34) |
| Q-12 | When do local models and Gemini arrive? | No; after Stage 8 unless the Operator pulls them earlier |
| Q-13 | How is escalation from a fast agent to a deliberate one modelled: two agents and a routing part, or a state machine? | S6.1 and Stage 7 |
| Q-14 | How does a requirement state an agent's required pass rate (needs constraint expressions or a convention on attributes)? | Stage 7 |
| Q-15 | Which code tools does the Assistant get for implementation, and how do the autonomy modes extend to side effects outside the System State? | Stage 8 (W8.2) |
| Q-16 | Where are recordings of agent answers stored (project folder or app data), and are they committed? | Stage 7 |
| Q-17 | Inter or Segoe UI Variable for the interface font, judged side by side at 100% and 125% scaling | Decided overnight 2026-09-27, pending Operator confirmation: keep Inter; the side-by-side judgment stays with the Operator (W5.2) |
| Q-18 | Server-side safety fallbacks under rig: an upstream contribution, a thin adapter, or none with "Retry with another model" | Decided overnight 2026-09-27, pending Operator confirmation: keep C-27 with a thin adapter in `agq-providers` for the `fallback` content block; the upstream contribution is written as a proposal, not filed |
| Q-19 | Should the default model move from `claude-opus-5` at `high` to `claude-opus-5-5` (cheaper; its default effort is `medium` per Anthropic's model documentation, not re-checked against the live page [10][12])? Newer models also tie thinking blocks to their model and conversation (S4.2, P2) | No; the evaluation set informs the Operator (R-45) |
| Q-20 | Do live evaluations of agents belong to the Simulation part (with a dependency on Providers) or to a separate part? | Stage 7 |

### 7.5 Decisions carried forward from the retired records

| ID | Decision | Origin |
|---|---|---|
| D-1 | Inheritance is a lookup over the element graph; inherited features are never copied into the specialising type. A reference (typing, specialisation, redefinition, connection end, satisfy) holds its target's identity once resolved, so renaming or moving the target never re-binds it by name. Relationships are properties of their owning element whose references carry the target's identity | ADR-0001, 0005, 0026; Stage 1 review |
| D-2 | Authored and implied facts stay separate. Implied relationships (implicit specialisation, implied redefinition) are derived, never written into the SysML text and never treated as authored | ADR-0001, 0026 |
| D-3 | Identity: retired identities are never reused; deleting and recreating gives a new element; identity is never re-matched by name. Library elements get identities derived from the pinned library bytes, so the identity file covers authored elements only | ADR-0006, 0012 |
| D-4 | An unresolved or wrongly typed reference never becomes a relationship. It stays a reported error at its source location; no placeholder targets | ADR-0006 |
| D-5 | Ambiguity is an error. It is never settled by identity, hash or traversal order, or "first import wins" | retired profile documents |
| D-6 | Caches are disposable. The durable truth is the SysML text plus the identity file; any cache is rebuilt from them | ADR-0028 |
| D-7 | Model and presentation are separate. Layout, camera, selection and views are presentation; removing an element from a view never deletes it; presentation undo never touches model history; diffs compare element identities | ADR-0029, 0030 |
| D-8 | The standard libraries are the pinned 2026-04 corrective release, not the original 2.0 downloads | standards discrepancies |

The ADRs named here are preserved at the tag `archive/pre-realignment`.

### 7.6 Decision log

| Date | Change | Why |
|---|---|---|
| 2026-09-27 | R-14 refined: Studio may depend on Assistant | The Conversation lives in the Studio and drives the Assistant |
| 2026-09-27 | R-15 implemented with the standard SysML `dependency` between part definitions, every edge listed, not transitive; temporary dependencies marked; checked by `tools/check_architecture.py` in CI | `dependency` is the standard concept for "requires"; explicit edges keep shortcuts visible |
| 2026-09-27 | Studio → LanguageCore and Assistant → LanguageCore allowed explicitly | Both use element identities and model types |
| 2026-09-27 | R-3 decided (rebuild) and R-6 adjusted (git2, `model/` plus `agentique.json`, merge by identity); D-1 reworded | Stage 1 spikes and review |
| 2026-09-27 | R-18 added with the System State interface | Stage 2 needs one rule for invalid edits |
| 2026-09-27 | Identity file format, deleted-target unbinding, saves writing only changed documents, repository only at the project folder, checkpoints refused when model files were edited outside the app | Stage 2 History |
| 2026-09-27 | Q-6 assessed overnight as "stay with egui"; Q-9 resolved as app-local conversation storage; Assistant defaults set | Stages 2 and 3 |
| 2026-09-27 | **The Operator confirmed** R-3, R-6 (as built), R-18, the identity file format, deleted-target unbinding, the save rules, Q-9 and the Assistant defaults (C-23 to C-27); **reopened** Q-6 (C-28); kept deviations 7–11 (C-30) | Discovery interview for this phase |
| 2026-09-27 | Stages 0–3 are accepted by a live run at the start of Stage 4 (C-29) | Stage 3 never ran against a live model |
| 2026-09-27 | This phase's direction, stage order, ambition, budgets, providers, keys, costs, autonomy, long tasks, memory, skills and agents confirmed (C-31 to C-46) | Discovery interview |
| 2026-09-27 | C-9 edited in place: "the Claude API first" replaced by several providers through rig (C-34, C-35) | Provider independence |
| 2026-09-27 | C-6 edited in place: asking on major decisions is the default autonomy mode; the Operator can waive it for a conversation (C-38) | Three autonomy modes |
| 2026-09-27 | C-14 edited in place: §1.6 keeps the original non-goals first and adds rows that follow from C-3, C-9, C-35, C-37, C-46 and Q-11 | This phase's scope |
| 2026-09-27 | C-22 edited in place: "retire" also covers preservation in git history, so `REALIGNMENT.md` is retired without a new tag (C-47) | Adoption of this document |
| 2026-09-27 | Stages renumbered: `REALIGNMENT.md`'s Stages 4 (simulation) and 5 (implementation links) become Stages 7 and 8; its later Stages 6 and 7 become 9 and 10 | C-31 puts the Studio, Settings and the agentic Assistant first |
| 2026-09-27 | `ROADMAP.md` replaces `REALIGNMENT.md` as the single governing text (C-47); `REALIGNMENT.md` is removed in W4.1 | One truth per topic (§8.2) |
| 2026-09-27 | W4.1: `REALIGNMENT.md` removed; `AGENTS.md` (rules of §8.1), `README.md`, `docs/stages.md`, `docs/subset.md`, the self-model, the architecture check, crate READMEs and source comments point here; stage references renumbered (simulation Stage 7, implementation links Stage 8) | C-47 |
| 2026-09-27 | **The Operator's overnight instructions** for Stages 4–8 (given on the evening of 2026-09-27): (1) no stopping at Operator gates: the evidence a gate asks for is produced and kept outside the repository, the stage is set to "provisionally complete, pending Operator acceptance", and a choice that is the Operator's takes this document's recommended (or the more conservative) option, recorded here as "Decided overnight 2026-09-27, pending Operator confirmation"; nothing is recorded as accepted or confirmed by the Operator that they did not accept or confirm; (2) exactly the three locked-core changes of §4.6 are authorised (`enum def` and enumeration values; the built-in `Agents` library; `dependency` in the subset), each recorded here when made, and no other; (3) S4.1 runs the automated gates G1–G8 only, with no switch to GPUI (that needs the Operator to accept the governance risk); Track A's code may be kept and merged as W5.1 after review, a recorded deviation from "throwaway"; Track B's kill check runs only with at least 12 GB of free disk; blind scoring waits for the Operator; (4) S4.2 builds the Anthropic, OpenAI and OpenRouter paths without keys, tested on canned streams and the local HTTP stand-in and marked "not tried live"; live parity checks run on DeepSeek; (5) Q-18 keeps C-27 with a thin adapter (Q-18 row); Q-17 keeps Inter; Q-10 is the app's local data (R-43); each decided overnight 2026-09-27, pending Operator confirmation (§7.4); anything else takes this document's recommendation; (6) live calls tonight run under a developer spend guard outside the product (C-37 is unchanged: the product has no spending limits) | The Operator is asleep; one stage finished well is worth more than several half-done |
| 2026-09-27 | **C-35 amended by the Operator:** DeepSeek joins the providers, with model `deepseek-flash` at effort `high` (the model offers `low`, `high` and `max`), as the only live test provider for the Assistant overnight; the product must work end to end with only a DeepSeek key configured. TypeSafe AI's Jev joins as a model provider for fast agents in designed systems, never for the Assistant. Q-11 resolved. §1.5, §1.6, §2.2 (L1), §2.4 (E4), §3.7, §4.1, §4.7, §4.8, §4.9, §4.11, §6.3, §6.7 and the glossary follow | The Operator tests with a DeepSeek key; Jev is exactly a fast agent's shape (§4.11) |
| 2026-09-27 | **C-34 clarified by the Operator:** Jev is a typed decision API that released rig does not support (`rig-typesafeai` is a `0.0.0` placeholder on crates.io [9]); it is implemented as a thin client inside `agq-providers`, behind the same boundary, until rig releases it | Provider neutrality without waiting for rig |
| 2026-09-27 | §4.8 gains DeepSeek and Jev columns, each capability verified from rig 0.42.0's source or the vendor's documentation and cited | C-35 as amended |
| 2026-09-27 | Stage completion during the overnight run is provisional: a stage built overnight is "provisionally complete, pending Operator acceptance" until the Operator has used it (C-15) | Overnight instructions, point 1 |
| 2026-09-27 | S4.2's provider layer is kept and merged in Stage 4 as the start of W5.7, a deviation from "throwaway" (§8.8); S4.2 gains P3a (live on DeepSeek). Decided overnight 2026-09-27, pending Operator confirmation | C-35 as amended: the Assistant must work end to end with only a DeepSeek key during Stage 4's live acceptance (W4.4) |
| 2026-09-27 | W4.7: CI checks the CPU-side budgets in release builds; budgets not met yet (scene update at 1k and 10k, W5.5) get interim ceilings at about three times today's measurements, recorded in §3.3; the budgets are unchanged | §8.6: never loosen a budget silently |
| 2026-09-27 | **S4.2 decided by its rule** (decided overnight, pending Operator confirmation): P1 and P5 pass; P3a (live on DeepSeek) works through the evaluation set; P2 and P3 not tried live (no keys); P4: native-tls; P6: a thin adapter for Anthropic's `fallback` block (Q-18). Proceed with rig for every provider (C-34); Anthropic moves onto it in W5.7 once P2 has been tried live with the Operator's key | §6.2 decision rule; the loss of an untried path is not assumed |
| 2026-09-27 | Interface 3 (tokens module, component list, gallery) becomes the first item of Stage 5, after W5.1, so it is written against egui 0.36; interface 4 (conversation format 2) is merged as a specification, since W5.7 is built in sequence, not fanned out (decided overnight, pending Operator confirmation) | Interfaces still merge before Stage 5 fans out |
| 2026-09-27 | Stage 4 recorded as provisionally complete, pending Operator acceptance (C-15); its live acceptance (W4.4) and that of Stages 0–3 (C-29) remain the Operator's | Overnight instructions, point 1 |
| 2026-09-27 | API keys for the overnight run live in a git-ignored `.env` (the Operator's `.gitignore` change, commit `400feced`); keys never enter the repository, logs, fixtures, recordings or pull requests (§8.7) | Key handling (R-25) |
| 2026-09-27 | S4.1 Track A built on a branch kept for W5.1 review: egui and eframe 0.36.2 with wgpu 30.0.1; the Rust toolchain moves from 1.92.0 to 1.97.1 (egui 0.36 needs 1.95; 1.97.1 was already installed); Inter's variable font replaces the three static weights; the Studio uses eframe's low-latency surface (one frame in flight), eframe's own default since 0.35. Decided overnight 2026-09-27, pending Operator confirmation | §7.6 exception for Track A (overnight instructions, point 3); egui's minimum Rust version |

### 7.7 The original requirements

The 20 requirements of the original specification (`Agentique-Specification-v0.1.html`,
kept as history) were reconciled in `REALIGNMENT.md` §6.5 (retained,
superseded or deferred, one line each). That reconciliation stands as recorded
on `main` at `6fc90b78`; its retained content is in this document (§4.2, §4.5,
§5.5, §8).

---

## 8. Rules against drift

### 8.1 Governing rules (`AGENTS.md` must state these)

1. **`ROADMAP.md` governs.** Every piece of work names the stage (§6) and the
   scenario step, decision (C-n) or recommendation (R-n) it serves. If it serves
   none, it does not start; propose an update to this document instead.
2. **Work only in the current stage.** Items that wait do not start early,
   however attractive.
3. **Read §1.3 before designing anything.** Prefer the simplest design;
   generalise instead of multiplying parts; use standard names; understand
   intent before acting literally; ask when intent is unclear.
4. **The stable core is locked** (R-16). Changing the language core, the System
   State operations or the persistence format requires an explicit Operator
   decision, recorded in §7.6.
5. **Changes that cut across parts start in the self-model.** Update
   `models/agentique/` first. The dependency check (R-15) must stay green.
6. **No new crate, top-level folder, register, generator or evidence format**
   without a distinct responsibility in the self-model and a reason the
   Operator can see.
7. **Standards are a reference, not a project.** Consult the pinned
   specifications to get concepts right. Extend the subset only for a scenario
   need and record it in the manifest. Record any deviation in one line with its
   reason.
8. **Preserve the pinned standards.** Never modify `standards/artifacts/`, the
   library archives and sources under `standards/`, the PDFs or
   `Agentique-Specification-v0.1.html`. Acquiring standards artifacts is never a
   build step.
9. **Run the checks** (`cargo fmt --all -- --check`,
   `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace`, `python tools/check_architecture.py`) before
   handing work back, and report the actual results honestly, including
   failures. Changes to the Studio or the Surface also run the reference budget
   run (§8.6).

### 8.2 One truth per topic

| Topic | Document |
|---|---|
| Direction, decisions and stages | `ROADMAP.md` |
| Stage progress | `docs/stages.md` |
| What Agentique is and how to run it | `README.md` |
| Architecture | `models/agentique/` |
| Supported KerML/SysML constructs and deviations | `docs/subset.md`, `docs/deviations.md` |
| Design tokens and components | the tokens module and the component gallery |
| Settings | the settings table in code |
| Budgets | §3.3 here, and the constants the harness asserts |
| A crate's purpose | its own short `README.md` |

- **When a decision changes, edit the governing text in place** and add a
  one-line entry to §7.6 saying what changed, when and why. Delete or rewrite
  superseded text. **Never leave competing versions side by side**; git history
  keeps the old one.
- **No versioned copies** of documents, profiles, registers or evidence
  directories (no `-v2`, `phase3`). Git provides versions.
- **Historical documents** are not kept in the working tree. Reference the
  archive tag or the commit instead.
- Any change to a requirement or decision keeps its origin (C-n or R-n) and
  states what it replaces.

### 8.3 How completion is demonstrated

- **A stage is complete when the Operator has used its outcome and accepts it**
  (C-15). Acceptance is recorded in one short stage summary in
  `docs/stages.md`: what was shown, what was measured, what failed, what is
  deferred.
- **Automated tests are necessary but never sufficient.** A green test suite
  does not mean a stage is done.
- **Report honestly.** "Works", "partially works", "not tried" and "failed" are
  different statements. A run completing is not a check passing. Never state
  that something works because a lot of code or evidence exists.
- **CI is green on a healthy tree.** A known failure is fixed, or the test is
  removed with a recorded reason. It is never left permanently red.
- **Do not commit** generated logs, screenshots, screenshot hashes, budget
  reports, evaluation results or transcripts, recordings made for measurement,
  or other machine-generated evidence. Evaluation task definitions and test
  fixtures are code and are committed.

### 8.4 Naming

- **Model concepts** use KerML/SysML names: part, port, interface, connection,
  item, attribute, requirement, state, action, specialisation, redefinition.
  **Agent** is the name of the built-in library definition for AI-driven parts
  (C-42); "Assistant" is Agentique's own agent.
- **Everything else** uses plain, widely understood software terms: change,
  branch, commit, view, validation error, undo, provider, skill, note, plan,
  compaction, evaluation. Do not use "candidate", "World", "publication",
  "receipt", "frontier", "authority", "fabric" or "rematerialization".
- **Product nouns** are fixed here: Studio, Surface, Panels, Conversation,
  Assistant, Orchestrator (later), System State, Settings, lock, scenario,
  simulation, implementation link, drift, agent, autonomy mode.
- A new product term needs a reason that no standard term fits, and an entry in
  the glossary (§9).

### 8.5 Design-system rules

1. Every colour, size, radius, duration and easing in UI code comes from a
   token. A check fails when a colour or size literal appears in UI code outside
   the tokens module.
2. A new component enters the gallery, in every state and both themes, before
   it is used.
3. One state vocabulary everywhere (§3.2, principle 4).
4. Colour carries state; shape, icon and badge carry kind.
5. Keyboard-invoked and frequent UI does not animate; all motion respects
   reduced motion.
6. A pull request that changes what the Operator sees names the journeys whose
   screenshots were reviewed. Screenshots are not committed.

### 8.6 Performance budgets

1. The budgets of §3.3 are part of correctness. CI enforces the CPU-side
   ceilings; the reference run enforces the rest before a Studio or Surface
   change merges and at every stage end.
2. A change that misses a budget is fixed or reverted, or the budget is changed
   here with its reason in §7.6. Budgets are never loosened silently.
3. Work that touches the frame path, the edit path or start-up comes with a
   measurement before and after.

### 8.7 Provider neutrality

1. Only `agq-providers` depends on rig, tokio, reqwest or the credential-store
   crates; the architecture check enforces it (R-41).
2. Code outside `agq-providers` asks the capability table, never a provider's
   name.
3. A feature available on one provider only degrades on the others as listed in
   §4.8; a new such feature adds its row there.
4. Keys leave the credential store only in the request to their own provider.
5. rig upgrades are deliberate: one pull request each, with five tasks of the
   evaluation set run on every Assistant provider.

### 8.8 Spikes and evaluations

1. A spike has a time box, fixtures, gates and a decision rule written before it
   starts. Its code is thrown away unless §7.6 records an exception; its
   decision is recorded in §7.6.
2. Evaluations cost money and need keys: they run on demand, never in the
   default test suite, and their results are shown to the Operator, not
   committed.

---

## 9. Glossary

| Term | Meaning |
|---|---|
| **Operator** | The person using Agentique |
| **Studio** | The Agentique application |
| **Surface** | The main visual, spatial view of the System State |
| **Panels** | Side panels for detail (Outline, Inspector, Requirements, Problems, History, later simulation and links) |
| **Conversation** | Chat with the Assistant |
| **Assistant** | The AI agent the Operator works with, with tools and skills; later the Orchestrator |
| **Orchestrator** | The later form of the Assistant that directs other assistants |
| **System State** | The live, authoritative KerML/SysML description of the system being built |
| **Lock** | A mark on a part meaning it changes only with the Operator's confirmation |
| **Settings** | The Studio view for the Operator's choices: providers and keys, models, the Assistant's behaviour, appearance, keyboard, projects |
| **Provider** | A service that runs models: Anthropic, OpenAI, OpenRouter, DeepSeek; TypeSafe AI (Jev) for fast agents |
| **Autonomy mode** | How much the Assistant asks before acting: ask before every change, ask on major decisions, or ask only on locks |
| **Plan card** | The Assistant's short plan for a task, shown in the Conversation and updated as steps finish |
| **Skill** | An instruction file (`SKILL.md`) the Assistant follows; built in or the Operator's own |
| **Note** | A short piece of information the Assistant proposed and the Operator approved, used in later conversations |
| **Compaction** | Summarising earlier conversation so the Assistant's context stays within limits |
| **Agent** | A part whose behaviour is produced by an AI model, in fast or deliberate mode; in the model, a part definition specialising `Agents::Agent`. The Assistant is Agentique's own agent. An agent's fast mode is unrelated to Anthropic's "fast mode", a faster serving option |
| **Fallback** | The deterministic part that takes over when an agent fails |
| **Stand-in** | A deterministic replacement for a model, in tests or simulation |
| **Recording** | A stored real answer of an agent's model, replayed in simulation |
| **Live evaluation** | Running an agent or the Assistant against a real model on several samples to measure a pass rate, apart from simulation |
| **Evaluation set** | The tasks used to measure the Assistant's behaviour against real models |
| **Design token** | A named value (colour, size, duration, easing) that all UI code uses |
| **Budget** | A numeric performance target that is checked continuously |
| **Reference run** | The budget run on the reference machine: the stress harness and journeys with budget assertions, before merging Studio or Surface changes and at stage ends |
| **Must-hold behaviour** | A behaviour of the Assistant that must succeed in every trial of the evaluation set, such as never changing a locked part without confirmation |
| **Follow mode** | A Surface option in which the camera moves to what the Assistant changes |
| **Reference machine** | The Operator's Windows machine, where frame, latency, start and memory budgets are measured |
| **Subset manifest** | The list of supported, partial and excluded KerML/SysML constructs, with reasons |
| **Deviation** | A recorded place where Agentique intentionally departs from the standard |
| **Scenario (simulation)** | A defined sequence of interactions run against the architecture |
| **Implementation link** | A link from a model element to code, tests or a running service |
| **Drift** | A detected difference between the model and a linked implementation |
| **Archive tag** | `archive/pre-realignment`, the preserved state before Stage 0 |

---

## 10. Sources

External sources were read on 2026-09-27. Repository evidence is cited by path
in the text. Where a source is a repository, the file paths are given where the
text depends on them.

**rig and Rust alternatives**

1. rig-core versions and dates on crates.io (0.42.0, 2026-08-17): https://crates.io/api/v1/crates/rig-core
2. rig source at tag `v0.42.0`, commit `d5a34986`: https://github.com/0xPlaygrounds/rig/tree/v0.42.0 — `crates/rig-core/Cargo.toml` (tokio, reqwest 0.13, features incl. `native-tls`); `crates/rig-core/src/providers/anthropic/completion.rs` (content blocks, `additional_params`, caching, stop reasons, usage, default `max_tokens`); `crates/rig-core/src/providers/anthropic/streaming.rs` (thinking, unknown blocks, tool input); `crates/rig-core/src/providers/anthropic/client.rs` (client-wide beta header); `crates/rig-core/src/providers/openai/responses_api/mod.rs`; `crates/rig-core/src/providers/openrouter/completion.rs`; `crates/rig-core/src/streaming/mod.rs` (`cancel()`); `crates/rig-agent/src/agent/run/mod.rs` (`AgentRun`); `crates/rig-agent/src/agent/completion.rs` (default turns); `crates/rig-agent/src/agent/prompt_request/streaming.rs` (errors end the stream); `tests/providers/anthropic/cassette/reasoning_tool_roundtrip.rs`
3. rig migration guide at `v0.42.0` (0.30 to 0.42, about 3,900 lines): https://github.com/0xPlaygrounds/rig/blob/v0.42.0/MIGRATING.md
4. rig, unreleased commits after 0.42.0 (110 on 2026-09-27): https://github.com/0xPlaygrounds/rig/compare/v0.42.0...main
5. rig issues #951, #1452, #2118, #2168, #2170, #2237: https://github.com/0xPlaygrounds/rig/issues/
6. reqwest 0.13.2 features (`rustls` uses aws-lc-rs): https://crates.io/api/v1/crates/reqwest/0.13.2
7. genai, a thin multi-provider client without an agent loop: https://github.com/jeremychone/rust-genai
8. Anthropic's public repositories (SDKs for other languages, none for Rust): https://api.github.com/orgs/anthropics/repos
9. rig's experimental `rig-typesafeai` crate on `main`: https://github.com/0xPlaygrounds/rig/tree/main/crates/rig-typesafeai ; on crates.io only a `0.0.0` placeholder (published 2026-09-21, read 2026-09-27): https://crates.io/api/v1/crates/rig-typesafeai

**Claude API**

10. Pricing and models: https://platform.claude.com/docs/en/about-claude/pricing
11. Thinking: display modes `summarized`, `omitted`, `updates`: https://platform.claude.com/docs/en/build-with-claude/thinking
12. Effort: https://platform.claude.com/docs/en/build-with-claude/effort
13. Prompt caching: https://platform.claude.com/docs/en/build-with-claude/prompt-caching
14. Compaction on demand: https://platform.claude.com/docs/en/build-with-claude/compaction-on-demand
15. Context editing: https://platform.claude.com/docs/en/build-with-claude/context-editing
16. Models API (list): https://platform.claude.com/docs/en/api/models/list
17. Messages API (temperature and determinism): https://platform.claude.com/docs/en/api/messages/create

**Agent loops**

18. Anthropic, Building effective agents: https://www.anthropic.com/engineering/building-effective-agents
19. Anthropic, Effective context engineering for AI agents: https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents
20. Anthropic, Writing effective tools for agents: https://www.anthropic.com/engineering/writing-tools-for-agents
21. Anthropic, Agent Skills, and the specification: https://www.anthropic.com/engineering/equipping-agents-for-the-real-world-with-agent-skills ; https://agentskills.io/specification
22. Anthropic, How we built our multi-agent research system: https://www.anthropic.com/engineering/multi-agent-research-system
23. Anthropic, Demystifying evals for AI agents: https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents
24. Claude Code, permission modes: https://code.claude.com/docs/en/permission-modes
25. Claude Code, interactive mode (queued messages, send now): https://code.claude.com/docs/en/interactive-mode
26. Claude Code, tools reference (task tools, tool output limits): https://code.claude.com/docs/en/tools-reference
27. Claude Code, memory: https://code.claude.com/docs/en/memory
28. Codex, agent approvals and security: https://learn.chatgpt.com/docs/agent-approvals-security
29. Codex source: `update_plan` prompt, compaction, compaction threshold: https://github.com/openai/codex/blob/main/codex-rs/core/gpt_5_2_prompt.md ; https://github.com/openai/codex/blob/main/codex-rs/core/src/compact.rs ; https://github.com/openai/codex/blob/main/codex-rs/protocol/src/openai_models.rs ; queued input: https://github.com/openai/codex/blob/main/codex-rs/tui/src/chatwidget/input_queue.rs
30. Gemini CLI: auto memory, chat compression, behavioural evaluations: https://github.com/google-gemini/gemini-cli (`docs/cli/auto-memory.md`, `packages/core/src/context/chatCompressionService.ts`, `evals/README.md`)
31. Cursor, agent overview (queue and send now): https://cursor.com/docs/agent/overview
32. Zed, agent panel (queue, steer, review, follow): https://zed.dev/docs/ai/agent-panel
33. Manus, Context engineering for AI agents: https://manus.im/blog/Context-Engineering-for-AI-Agents-Lessons-from-Building-Manus
34. OpenAI Agents SDK, default maximum turns: https://github.com/openai/openai-agents-python/blob/main/src/agents/run_config.py

**Settings and secrets**

35. VS Code, settings: https://code.visualstudio.com/docs/configure/settings
36. VS Code, natural-language settings search: https://code.visualstudio.com/blogs/2018/04/25/bing-settings-search
37. Zed, configuring Zed and API keys: https://zed.dev/docs/configuring-zed ; https://zed.dev/docs/ai/use-api-access
38. Zed on Windows, credential storage: https://github.com/zed-industries/zed/blob/main/crates/gpui_windows/src/platform.rs
39. GitHub, sudo mode and deleting a repository: https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/sudo-mode ; https://docs.github.com/en/repositories/creating-and-managing-repositories/deleting-a-repository
40. GitHub CLI, `gh auth login` (credential store, plain-text fallback, `GH_TOKEN`): https://cli.github.com/manual/gh_auth_login
41. Primer, saving pattern: https://primer.style/product/ui-patterns/saving/
42. Cloudscape, delete patterns: https://cloudscape.design/patterns/resource-management/delete/
43. Microsoft, guidelines for app settings: https://learn.microsoft.com/en-us/windows/apps/design/app-settings/guidelines-for-app-settings
44. Raycast, bring your own key: https://manual.raycast.com/ai/bring-your-own-key
45. Jan, custom endpoints (testing keys): https://jan.ai/docs/desktop/remote-models/custom-endpoint
46. `keyring` 4.2.0, `keyring-core` 1.0.0, `windows-native-keyring-store` 1.1.0: https://crates.io/crates/keyring ; https://docs.rs/keyring-core ; https://docs.rs/windows-native-keyring-store
47. Windows `CREDENTIALW` (2,560-byte secrets, persistence): https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw
48. Windows `SystemParametersInfo` (`SPI_GETCLIENTAREAANIMATION`): https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfow

**Visual and interaction design**

49. Linear, How we redesigned the Linear UI: https://linear.app/now/how-we-redesigned-the-linear-ui
50. Linear, Behind the latest design refresh: https://linear.app/now/behind-the-latest-design-refresh
51. Linear, Quality Wednesdays: https://linear.app/now/quality-wednesdays
52. Linear, agent best practices: https://linear.app/developers/agent-best-practices
53. Raycast, action panel and tool confirmations: https://manual.raycast.com/action-panel ; https://developers.raycast.com/api-reference/tool
54. Figma, UI3 and zoom shortcuts: https://www.figma.com/blog/our-approach-to-designing-ui3/ ; https://help.figma.com/hc/en-us/articles/360041065034-Adjust-your-zoom-and-view-options
55. Figma, Keeping Figma fast: https://www.figma.com/blog/keeping-figma-fast/
56. tldraw, grid: https://raw.githubusercontent.com/tldraw/tldraw/main/packages/editor/src/lib/components/default-components/DefaultGrid.tsx
57. Unreal Engine, connecting nodes and graph level of detail: https://dev.epicgames.com/documentation/en-us/unreal-engine/connecting-nodes-in-unreal-engine ; https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Editor/GraphEditor/EGraphRenderingLOD__Type
58. ComfyUI, canvas settings (low-detail zoom): https://docs.comfy.org/interface/settings/lite-graph
59. IcePanel, tags and flows: https://docs.icepanel.io/visual-storytelling/perspective-tags ; https://docs.icepanel.io/visual-storytelling/flows
60. Warp, blocks: https://docs.warp.dev/terminal/blocks/block-basics
61. Zed, performance posts: https://zed.dev/blog/videogame ; https://zed.dev/blog/120fps
62. Agent Client Protocol, tool calls: https://agentclientprotocol.com/protocol/tool-calls
63. Radix Colors: https://github.com/radix-ui/colors/blob/main/src/dark.ts ; https://github.com/radix-ui/colors/blob/main/src/light.ts
64. Fluent 2 motion, durations and curves: https://fluent2.microsoft.design/motion ; https://github.com/microsoft/fluentui/blob/master/packages/tokens/src/global/durations.ts ; https://github.com/microsoft/fluentui/blob/master/packages/tokens/src/global/curves.ts
65. Windows typography: https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/typography
66. Primer, size primitives: https://primer.style/foundations/primitives/size
67. Responsiveness guidance: https://web.dev/articles/rail ; https://lawsofux.com/doherty-threshold/
68. On animating frequent actions: https://emilkowal.ski/ui/great-animations ; https://rauno.me/craft/interaction-design
69. AI SDK Elements, tool component: https://elements.ai-sdk.dev/components/tool

**UI toolkits**

70. egui changelog (0.33 to 0.36.2) and versions: https://github.com/emilk/egui/blob/main/CHANGELOG.md ; https://crates.io/api/v1/crates/egui
71. egui, selection across labels (deselects when not visible): https://github.com/emilk/egui/blob/main/crates/egui/src/text_selection/label_text_selection.rs
72. egui: subpixel text #2354, live regions #2647, no inline widgets in text (discussion #3781), animation APIs (`context.rs`): https://github.com/emilk/egui/issues/2354 ; https://github.com/emilk/egui/issues/2647 ; https://github.com/emilk/egui/discussions/3781 ; https://github.com/emilk/egui/blob/main/crates/egui/src/context.rs
73. egui, font variations #7859 and shaping #8031: https://github.com/emilk/egui/pull/7859 ; https://github.com/emilk/egui/pull/8031
74. GPUI on crates.io (0.2.2, 2025-10-22) and `gpui-pre` snapshots: https://crates.io/api/v1/crates/gpui ; https://crates.io/api/v1/crates/gpui-pre
75. GPUI's Windows renderer and macOS-only surface painting: https://raw.githubusercontent.com/zed-industries/zed/main/crates/gpui_windows/src/directx_renderer.rs ; https://github.com/zed-industries/zed/pull/64306
76. gpui-ce, custom wgpu rendering: https://github.com/gpui-ce/gpui-ce/pull/237
77. "Major brakes" on GPUI development (Hacker News): https://news.ycombinator.com/item?id=47003569
78. Zed screen-reader support: https://github.com/zed-industries/zed/issues/41138 ; https://github.com/zed-industries/zed/discussions/6576
79. GPUI Kit and its pinned dependency: https://github.com/longbridge/gpui-kit ; https://crates.io/api/v1/crates/gpui-component
80. Slint, wgpu integration and styled text: https://docs.rs/slint/latest/slint/wgpu_30/index.html ; https://docs.slint.dev/latest/docs/slint/reference/elements/styledtext/
81. Iced accessibility (open since 2020; an AccessKit pull request closed): https://github.com/iced-rs/iced/issues/552 ; https://github.com/iced-rs/iced/pull/3281

**Agents, fast and deliberate modes**

82. OMG, SysML v2.0 Part 1, formal/2026-03-02 (pinned `SysML.pdf`): clauses 7.6.8, 7.11.1, 7.21, 7.27.3–7.27.4
83. OMG, KerML 1.0, formal/2026-03-01 (pinned `KerML.pdf`): clause 7.4.13
84. SysML v2 training, "41. Language Extension": https://github.com/Systems-Modeling/SysML-v2-Release/tree/master/sysml/src/training/41.%20Language%20Extension
85. Simplex (Sha, IEEE Software 2001) and run-time assurance per ASTM F3269 (Nagarajan et al., AIAA SciTech 2021): https://ieeexplore.ieee.org/document/936213/ ; https://arc.aiaa.org/doi/10.2514/6.2021-0525
86. EASA AI Concept Paper, Issue 02: https://www.easa.europa.eu/en/downloads/139504/en
87. DARPA Assured Autonomy (learning-enabled components): https://www.darpa.mil/research/programs/assured-autonomy
88. Deterministic model stand-ins: https://pydantic.dev/docs/ai/guides/testing/ ; https://openai.github.io/openai-agents-python/testing/
89. VCR.py record modes: https://vcrpy.readthedocs.io/en/latest/usage.html
90. Thinking Machines, Defeating nondeterminism in LLM inference: https://thinkingmachines.ai/blog/defeating-nondeterminism-in-llm-inference/
91. OpenAI, GPT-6 Luna and pricing: https://developers.openai.com/api/docs/models/gpt-6-luna ; https://developers.openai.com/api/docs/pricing
92. Gemini API pricing: https://ai.google.dev/gemini-api/docs/pricing
93. Artificial Analysis, leaderboard, Claude Haiku 4.5 and GPT-6 Luna (figures indicative): https://artificialanalysis.ai/leaderboards/models ; https://artificialanalysis.ai/models/claude-4-5-haiku ; https://artificialanalysis.ai/models/gpt-6-luna
94. Booch et al., Thinking Fast and Slow in AI (SOFAI), AAAI 2021: https://ojs.aaai.org/index.php/AAAI/article/view/17765
95. RouteLLM and FrugalGPT: https://arxiv.org/abs/2406.18665 ; https://arxiv.org/abs/2305.05176
96. Figure Helix and Helix 02; NVIDIA GR00T N1: https://www.figure.ai/news/helix ; https://www.figure.ai/news/helix-02 ; https://arxiv.org/abs/2503.14734
97. TypeSafe AI, Introducing System One Models and Jev (2026-09-15): https://typesafe.ai/blog/introducing-system-one-models-and-jev
98. TypeSafe AI documentation (API, models and prices, confidence, Jev 1.13 variability): https://docs.typesafe.ai/api ; https://docs.typesafe.ai/models ; https://docs.typesafe.ai/confidence ; https://docs.typesafe.ai/model-jaggedness/jev-1.13 ; https://evals.typesafe.ai/
99. Hacker News launch discussion (including why it does not fit the OpenAI-style API): https://news.ycombinator.com/item?id=49717558
100. OpenRouter, Jev listing and public model list: https://openrouter.ai/~typesafe/jev-latest ; https://openrouter.ai/api/v1/models
101. An independent test of Jev for intent routing: https://suraj-website-eta.vercel.app/blog/what-a-correct-decision-costs

**Additional**

102. VS Code, the agent's request limit per turn: https://github.com/microsoft/vscode/blob/main/src/vs/workbench/contrib/chat/browser/chat.shared.contribution.ts
103. Devin, knowledge suggestions: https://docs.devin.ai/product-guides/knowledge.md
104. Apple Human Interface Guidelines, dark mode: https://developer.apple.com/design/human-interface-guidelines/dark-mode
105. DeepSeek API documentation (read 2026-09-27): pricing https://api-docs.deepseek.com/quick_start/pricing ; chat completions https://api-docs.deepseek.com/api/create-chat-completion ; thinking mode https://api-docs.deepseek.com/guides/thinking_mode ; tool calls https://api-docs.deepseek.com/guides/tool_calls ; balance https://api-docs.deepseek.com/api/get-user-balance ; the model list `GET https://api.deepseek.com/models` (read by the Operator on 2026-09-27)
