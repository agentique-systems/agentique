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

**Purpose (C-55).** This paragraph is the one authoritative statement of
Agentique's purpose; every other document, instruction and agent brief refers
to it here instead of restating it. **Agentique is a system for understanding,
modelling, simulating, verifying, implementing and evolving systems through
explicit architecture grounded in KerML and SysML v2, in their standard terms
(§8.4). Agentique is itself such a system:** it uses for its own
development the same modelling concepts, operations, contracts and
verification mechanisms it gives its users. AI
agents do the work; people express intent, inspect the system visually,
observe execution and evidence, and keep control of the purpose and of
consequential decisions. The ambition is a system of systems: a system that
helps build and evolve other systems, itself included, through recursive
composition and reusable knowledge. Autonomous development is a means of
improving this modelling and engineering environment, and every improvement
serves this purpose.

Two architectural principles follow, for Agentique and for every system built
with it:

- **The root system.** A system's root states its purpose, a small set of
  coherent responsibilities and stable contracts. A system that builds systems
  is composed the same way as the systems it builds, and refers to them, and
  to other instances of itself, rather than containing them. The core stays
  cohesive and limited; no part becomes a universal object that accumulates
  every responsibility.
- **Generalise the mechanism; specialise the application.** Shared contracts
  are definitions and contextual roles are usages; variation (providers,
  domain rules, workflows, implementation choices) sits at the narrowest scope
  that needs it, through specialisation, subsetting and redefinition with
  their standard meanings, explicit interfaces, and dependencies directed
  toward stable abstractions. An abstraction is introduced when distinct uses
  demonstrate the commonality; meaningful differences are kept rather than
  hidden behind flags, untyped payloads or generic dispatchers. A design is
  simpler when the whole is easier to understand, less coupled, less
  duplicated, cheaper to change and to verify, not when it has fewer boxes or
  lines. Compositionality, cohesion, low coupling, substitutability,
  separation of concerns, dependency inversion and behavioural refinement are
  the criteria.

The self-model holds this purpose as the requirement `purpose` of
`model/Agentique.sysml`, whose subrequirements state its obligations (W13.4).
The purpose and its **protections** change only by the Operator's decision:
`ROADMAP.md` (this paragraph with it) and the requirement `purpose` with
everything it owns change by a decision recorded in §7.6, never inside an
objective, even one that names them; the agent configuration (`AGENTS.md`,
`CLAUDE.md`) and the code of the gates of §4.16 (the locked Orchestrator)
change inside an objective only when the Operator's objective names them
(C-53); and only the Operator moves the approved baseline. Agents may propose
such a change, never make it (§4.16).

**How it is delivered.** Agentique is a **native desktop application in which
a person and AI agents design, simulate and implement systems together,
working at the level of system architecture rather than code.**

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
  tools and skills to understand and change the system. Its sessions also do
  the work of the Operator's objectives, which the **Orchestrator** directs
  (C-5, C-53).

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
  locked part requires the Operator's explicit confirmation of that change, or
  an objective that names it (C-11, C-53).

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

**The factory loop (C-50, 2026-09-30).** The Operator directed that agents in
the model, simulation and implementation are built as **one continuous loop**,
not three features: intent → architecture → scenarios → execution →
implementation → checks → informed change. The Operator works at the level of
structure, contracts, behaviour and outcomes; Agentique and its Assistant take
on more of the implementation while keeping that structure visible and
protected. Its long-term direction is an architecture-led software factory; this
phase delivers the **first complete, inspectable loop** (§6.5, Scenario I in
§2.10), not the whole platform. The promise it serves: the Operator can
understand what the system is supposed to do, explore how it behaves, produce
an implementation, and see where the implementation no longer agrees with the
design. The model (intent), the implementation (actual code) and run results
(observations under stated conditions) stay three separate things; none is
silently changed to agree with another (§4.14, §4.15).

**Agentique builds Agentique (C-51, 2026-10-01).** The Operator directed the
last externally driven bootstrap: Agentique becomes the place where the
Operator understands, maintains and develops Agentique itself (Scenario C,
§2.8; Stage 10, brought forward, §6.6). The organising principle for this and
later work: **Agentique is a workspace where the Operator understands and
deliberately changes a system through its architecture, while AI helps build
and check its implementation.** (Under C-53, below, AI agents become the
workspace's primary users, while the Operator gives intent and watches; C-55
replaces this principle with the purpose of §1.1.)
Understanding and deliberate change are the centre; the Library, simulation, implementation, evaluations and the
Assistant serve one system model and one working experience. Self-development
follows understand → propose → approve → change → check → review → build →
try → adopt, never "an AI rewriting its own application" (C-53 below
removes the Operator's approval of each cycle, not the checks). Three outcomes are
inseparable: Agentique becomes understandable again (its self-model explains a
working product), its Assistant gains a Claude Agent SDK runtime, and it can
produce, validate and adopt its next version, and recover from a bad one. The
measure of improvement is that less knowledge is needed to understand and
safely change a relevant part, not line counts.

**Agentique improves itself (C-53, 2026-10-03).** The Operator directed the
next evolution: **autonomous, self-improving development through Agentique
itself**. AI agents become Agentique's primary users. The Operator supplies
intent as an **objective**, observes activity and outcomes in the Studio, and
steers or stops the work; routine approvals and terminal repair are no longer
the Operator's. Agentique builds, runs, tests and evaluates Agentique, and
adopts a new version when the objective's acceptance criteria pass. What
keeps this from being slop is unchanged in kind and stronger in force: model
changes still go through the one operation boundary with validation and
identity; deterministic checks and independent review decide, and an agent's
judgment never overrides a failing check; weakening a check never counts as
improvement; every step is recorded, and a known-good version with its data
is kept to return to (§4.16, Stage 11 in §6.8). Self-development becomes
understand → propose → implement → check → review → merge → build → try →
adopt → continue, run by the Orchestrator with the Operator watching.

**Agentique tests and improves itself (C-54, 2026-10-04).** The Operator
directed the next stage: from one intent, Agentique **explores its own running
application**, reproduces what it finds, carries a reproduced problem through
a cycle to adoption, and explores the adopted version again with what it
learned, so the loop becomes intent → exploration → reproduced finding →
proposal → implementation → independent checks and review → merge → build,
try, adopt → exploration informed by the last run. Each agent role runs on
the model suited to it (C-54), agents may visibly delegate work to other
agents within the Operator's permissions and budgets, and the Operator
watches agents type, click and decide in visible windows. Progress is
measured, not assumed: coverage reached, findings reproduced, fixes adopted,
and the cost and latency of each way of deciding. Exploration finds problems
only through deterministic checks that fail and reproduce, never through a
model's opinion alone; a fix counts only when its regression fails on the
original implementation (§4.16, Scenario K in §2.12, Stage 12 in §6.9).
External Claude Code bootstraps this stage and is not needed afterwards for
ordinary development, testing, orchestration or adoption.

**Agentique models, simulates and evolves systems, itself included (C-55,
2026-10-09).** The Operator recorded the enduring purpose (§1.1) and
directed the next stage: strengthen the KerML/SysML foundations the purpose
rests on, and make autonomous evolution stay faithful to it. The supported
subset grows where engineering scenarios need it (referential usages for
shared elements, requirement constraints with their assumptions and their
evaluation, reusable calculations), each complete from text to validation,
System State operations, the Surface and the agents' tools, execution or
analysis, evidence, and saving; the self-model states the purpose's
obligations as requirements and models the autonomous lifecycle (objectives,
delegation, execution, verification, review, adoption, recovery,
continuation) as behaviour that runs and is checked against the
implementation; a second, materially different system (an inspection drone
and its charging station) exercises the same abstractions; every autonomous
proposal names the requirement it serves, the elements it affects, its
benefit, its evidence and its effect on root complexity, and is reviewed
against the Operator's approved baseline, not only the previous commit;
verification claims stay apart, each shown no stronger than it is (§4.14);
and the remaining proof of Stage 12 runs with a modelling objective (§6.10).

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
| An autonomous multi-agent factory for arbitrary projects | Not yet; the Orchestrator runs objectives on Agentique's own repository first (C-5, C-53); agents delegate child objectives only within one objective's permissions and budgets (C-54) |
| A physics or physical-systems simulator | Not the aim; simulation tests architecture and contracts (C-16) |
| A generator of verification paperwork for its own sake | Never; proof is working software the Operator uses (C-15) |
| An agent framework or runtime for other people's code | Not a goal; agents in designed systems are implemented in the project's own code (C-46) |
| A chat product with the architecture on the side | Never; the Surface stays an equal way to work (C-3) |
| Local models and Gemini | Not in this phase (C-35); the provider layer keeps the door open (Q-12) |
| Typed fast-decision APIs other than Jev | Not in this phase; Jev is a model provider for fast agents (C-35, Q-11) |
| Spending limits on the Operator's own conversation | Not in this phase; costs are shown, not capped (C-37). An objective's work is bounded because nobody approves each step (C-53): by its cycles, attempts, exploration steps and model calls, lack of progress and the Operator's Stop; spend and time are unlimited unless set (C-54 as amended, §7.6) |
| A marketplace, cloud registry, package manager or vendor catalogue of building blocks | Not a goal; the Library is local, and a project keeps its own copies of what it uses (C-49) |
| Unattended production deployment, destructive migrations or irreversible external actions; remote worker fleets; a security sandbox Agentique does not have | Not in this phase; implementation work is local, bounded by an objective's permissions and budgets, and the isolation the host really offers is reported as it is (C-50, C-53) |
| General physical or numerical simulation, broad language support for implementation, a general-purpose workflow language | Not in this phase; the run contract stays open to them (C-50, §4.14) |
| An AI that changes its own application without deterministic checks, independent review, a visible record, a way to stop it and a known-good version to return to; a plugin marketplace or remote tool platform for the Assistant; an enterprise deployment system for Agentique's own builds | Never for the first; not a goal for the others. Self-improvement runs as objectives the Operator gives, watches and can stop; checks and review decide, and adoption keeps the last known good build (C-51, C-53) |

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

### 2.8 Scenario C: Agentique builds Agentique (Stage 10, C-13, C-18, C-51)

Agentique's own repository is an ordinary project whose model is Agentique's
self-model (§4.6). The Operator opens it with "Develop Agentique", understands
it, and changes Agentique through the Assistant and a task, then builds, tries
and adopts the next version, and does it again from that version. Its locked
core stays intact. Reading an unmodelled codebase into a model (Q-7) stays
later; Agentique's own model is written by hand.

| Step | What the Operator does | Observable success | Failure and recovery behaviour |
|---|---|---|---|
| C1 | **Understand.** Chooses "Develop Agentique" and explores purpose and boundaries, then the parts, their interactions and their code. | The Surface shows Agentique's parts and the interactions between them; the Inspector answers for any part what it is, why it exists, what it owns, what depends on it, where it is implemented and what changing it affects, with links that open the code and the checks; navigation keeps place and history. | A part without implementation links says so; nothing is invented to fill it. |
| C2 | **Follow a workflow.** Runs the scenarios "Rename a part", "The Assistant changes the model" and "A development task", with their failure paths. | Each runs in model execution and steps across the parts on the Surface; each step names the code that does it and the tests that cover it; the result says it checked the architecture model, and which code checks exist. | A step without a linked test is shown as not covered by code checks. |
| C3 | **Ask the Assistant.** Asks it to explain a part or a workflow. | It answers from the same model and links, citing elements and source locations, and says where it is unsure. | Without a key it says how to add one; manual work goes on. |
| C4 | **Define a task.** Asks for a bounded improvement; the Assistant proposes a task. | The task names the requested outcome, affected parts, protected boundaries, source and model bases, permitted writes, required checks and acceptance criteria, and its architectural consequences before any code changes. Approval fixes them. | The Operator can decline or narrow it; a local correction never comes back as a redesign. |
| C5 | **Implement and check.** The Assistant works in the task's worktree, proposes model changes there with ordinary operations, runs the required checks and repairs within bounds. | The Conversation says whether it is inspecting, proposing, editing the working copy, running checks, waiting or presenting; every required check ends with an explicit outcome. | Repeated failure ends the attempt with what blocks it; a required check not run is never a pass. |
| C6 | **Review and integrate.** Reviews the model changes and the code changes together, with the check results, and integrates. | Exactly the reviewed commit is integrated; unrelated uncommitted work stays as it was; changes to locked parts' code or to safeguards ask explicitly. | A moved base, a stale verification or a changed working copy blocks integration with the reason. |
| C7 | **Build, try and adopt.** Builds the integrated revision, tries it as a test instance, then chooses "Use this build". | The build names its source revision, checks and changes; the test instance runs with its own data; after adoption Agentique restarts in the new version and the development project resumes. | A build that does not start returns to the last known good version with a banner and diagnostics; data is backed up first; a build that changes a data format cannot be adopted yet. |
| C8 | **Repeat.** From the adopted version, completes a second, independent improvement in another area. | The same loop works from the new version, without external editing or a terminal. | External repair, if ever needed, is recorded as such. |

**Stage mapping.** C1–C8 are Stage 10 (§6.6). The proof is the Operator's two
generations (gates A–E in §6.6); C-53 replaced gates C and D with Stage 11's
autonomous proof (W11.7, §6.8).

### 2.9 Scenario H: reusing building blocks (Stage 5, C-49)

The Operator builds with reusable definitions instead of modelling common
structure from scratch. A building block is an ordinary definition (§4.13);
the Library finds it in the built-in library, the project and My Library.

| Step | What the Operator does | Observable success | Failure and recovery behaviour |
|---|---|---|---|
| H1 | Opens the Library beside the Outline (Ctrl+Shift+L) and types "cache". | Results change with each key (within one frame at 5,000 blocks, §3.3), each showing its kind, source, purpose and public ports; the arrow keys move through them; a preview shows the selected block's ports, inner parts and connections. | No match shows the nearest names. |
| H2 | Inserts `CachedStore` into a part: Enter, "Insert from Library…" in the palette, or dragging it onto the Surface. | One card appears, typed by the block and showing its ports; the definitions it needs are copied into the project's `Library` package in the same change; the new card is selected with its name ready to edit. History says "Add cachedStore : CachedStore from the Library". | Inside a locked part the lock prompt asks first; Ctrl+Z removes all of it in one step. |
| H3 | On a port, chooses "What can connect here?", then inserts and connects a block. | Only blocks with a port that fits by the model's own rule are offered; the usage and its connection arrive in one change. | A block that does not fit is not offered; a refused connection says why. |
| H4 | Opens the usage's definition (Enter or double-click) and goes back (Backspace or Alt+←). | The Surface shows the definition's inside under a breadcrumb; Back returns to the usage where it was. | — |
| H5 | Changes a port or attribute shown on the usage through its type. | The Studio asks whether to change the definition (naming every usage it changes), override it in this usage only, or create a specialisation; the definition changes only when chosen. | Cancel leaves everything as it was. |
| H6 | Selects connected parts and chooses "Create building block from selection". | A preview lists the new definition's ports and the connections that will pass through them; one undoable change replaces the parts with a usage of the new definition, and the parts keep their identity. | A connection that reaches a part without a port is named, and nothing changes until it is fixed. |
| H7 | Saves a definition to My Library, opens another project and inserts it there. | The other project gets its own copies: it validates without My Library, and changing My Library later changes no project. | A different definition with the same name is never overwritten: the Operator uses the project's, copies under another name, or cancels. |
| H8 | Asks the Assistant for a cached store, and later for something no block fits. | For the first, the Assistant searches and reads the Library and uses the block in one visible, undoable change; for the second, it models a plain project definition instead of forcing a block. | Saving to My Library needs the Operator's request and a visible confirmation. |

**Stage mapping.** H1–H8 are W5.13 (§6.3). The journey `h-library` covers
them without the network, with a scripted stand-in for H8 (R-47).

### 2.10 Scenario I: the factory loop, end to end (C-50)

The URL shortener gains an AI-driven link-screening component with a
deterministic recovery path and a rule for links that need review. Its
**acceptance behaviour is fixed here, before any implementation is generated**:

1. A shortened link is first screened by the agent `LinkScreening` (fast mode,
   `minConfidence` 0.8, `maxLatencyMs` 500), which answers a verdict: `allow`,
   `review` or `block`, with a confidence and a reason.
2. `allow` at or above `minConfidence` stores the link **active**; the answer
   carries its code and status `active`.
3. `review`, or any answer below `minConfidence`, stores the link **held for
   review** (status `held`); `block` stores nothing (status `blocked`, no code).
4. When the agent fails (timeout, invalid output, refusal, a tool it cannot
   reach) or is below `minConfidence`, its deterministic fallback
   `BlocklistScreening` decides: a host on the blocklist is blocked, anything
   else is held for review. The service never activates a link without either
   a confident `allow` or a reviewer's approval.
5. Resolving a code redirects only for an active link; a held or unknown code
   does not redirect.
6. A reviewer's approval of a held link makes it active; a rejection blocks it.
7. The public API (the `api` part's ports and the items they carry) is locked.
   The guardrail requirement `ReviewBeforeActivation` ("a link that needs
   review is never activated without an approval") has the API as subject.

| Step | What the Operator does | Observable success | Failure and recovery behaviour |
|---|---|---|---|
| I1 | **Design.** Assembles the system with Library blocks (a gateway API, a store, the screening contract, an agent and its fallback), sets the agent's configuration and locks the public API. | The agent card shows its badge and mode; the Inspector shows its contract (what it may observe and do, what output is acceptable, what happens when it cannot answer), configuration and fallback. | A fallback that does not share the agent's contract, or that is itself an agent, is a validation error at the agent. |
| I2 | **Exercise the model.** Runs the successful, blocked, timed-out, invalid-output and review-required scenarios with deterministic stand-ins. | Each run steps through the parts on the Surface from recorded events; the trace says what happened where and why it stopped; checks are listed apart from run completion as passed, failed, not run, unsupported, blocked or inconclusive. | A missing behaviour, stand-in or recording stops the run with that reason at the element concerned; nothing is guessed. |
| I3 | **Evaluate the agent.** Replays recorded cases without the network; then explicitly requests a live evaluation on a chosen provider and model. | Replay is deterministic and says which recording answered. The live evaluation reports per case and per check the sample count, outcomes, failure categories and an uncertainty interval, with the provider, model, instructions and case versions; a good run can be kept as recordings. | A missing recording stops the replay with `missing-recording`; it never falls through to a live call. A live evaluation not run is shown as not run. |
| I4 | **Implement.** Asks the Assistant to implement the system in a separate repository. | The Assistant shows a plan and its scope, works in an isolated worktree, builds, runs the checks, repairs within scope, and presents a patch beside the affected parts and contracts. After integration the code is linked to the model and the contract scenarios run against the real code. | A contract change the implementation seems to need returns to the Operator as an architecture decision, never inside a patch. |
| I5 | **Detect a contradiction.** Deliberately breaks the implementation so that a held link becomes active. | The protected check fails in the implementation run, is shown as drift at the part, and links to the step, the trace and the code. | — |
| I6 | **Repair safely.** Asks the Assistant to fix it. | The Assistant inspects the failure, proposes a bounded repair, applies it in the worktree and reruns the check; the requirement, the protected tests and the public API are unchanged. | Repeated failure or no progress ends the attempt with a report of what blocks it. |
| I7 | **Handle changed intent.** Changes an approved setting (for example `minConfidence`). | The results that depend on it are marked outdated at once; the Assistant updates the implementation deliberately; new runs produce current results. | An old passing result is never shown as current evidence. |
| I8 | **Recover.** Cancels a running job, reopens the project and resumes. | Earlier results and the job's history are there; resuming never repeats a side effect that completed. | A job interrupted by a crash is shown as interrupted, with what completed and what did not. |

A second, smaller conventional example (a retrying notification dispatcher,
with other names and configuration) runs through the same model runner and
implementation adapter. For one scenario the model and the implementation are
each broken on purpose, and each break is caught by its own execution path. A
supported implementation-link check (crate dependencies against the self-model)
runs on Agentique's own repository.

**Stage mapping.** Scenario I is Stages 7–8 (§6.5). It takes over A5–A7 and
G4–G9; G1–G3 (agents in the model) move from Stage 6 into it.

### 2.11 Scenario J: Agentique improves itself (C-53)

Agentique's own repository is open ("Develop Agentique"). The Operator gives an
objective and watches; agents do the work in the visible application.

| Step | What happens | Observable success | Failure and recovery behaviour |
|---|---|---|---|
| J1 | **Intent.** The Operator writes an objective in the Objectives panel ("Fix a correctness problem in …", "Make … easier to understand"); what it does (its improvements, whether it merges and adopts) is read from the intent, and the Operator starts it. | What it was read as, by whom, and the permissions it carries are shown and saved before any agent starts. | A missing key, runtime or repository says what is missing; nothing starts half-configured. |
| J2 | **Understand and propose.** A lead agent inspects the architecture, the code and the running application, and proposes one improvement with acceptance criteria and a plan. | The proposal names the parts it affects, why, the acceptance criteria (each with how it is checked) and the plan; the criteria are frozen for the cycle. | A proposal without a checkable criterion is refused and asked again, within the attempt budget. |
| J3 | **Implement.** An implementer agent works in the cycle's worktree with the full development tools, changing the model only through Agentique's operations. | The Studio shows which agent is doing what (reading, editing, running checks), with the files and elements involved. | A file tool's write outside the worktree or to a protected path is refused with the reason; commands are not confined, so a change touching a path it may not is refused before merging (§4.16). |
| J4 | **Check and evaluate.** Deterministic checks run on a clean checkout; the changed Agentique is built and started as a test instance, and an evaluator agent operates it through the control interface and checks the behavioural criteria. | Every required check and criterion ends with an explicit outcome; the evaluator's actions are visible in the test instance's window and in the activity record. | A failing check is repaired within bounds; the same failure twice, or no progress, ends the cycle with a blocker report. A required check not run is never a pass. |
| J5 | **Review.** An independent reviewer agent, with fresh context and no write access, reviews the change against the criteria and the baseline. | Its verdict and findings are recorded; findings go back to repair. A change to tests or requirements is named and judged explicitly. | Weakening or removing a check to make it pass is refused as an improvement. |
| J6 | **Merge.** The cycle's branch is pushed, a pull request opened, the repository's checks awaited and the reviewed commit merged. | The pull request, its checks and the merged commit are linked from the cycle. | Red or missing checks, a moved base or a changed commit block the merge with the reason. |
| J7 | **Build, try, adopt.** The merged commit is built, verified against its source, started as a test instance, exercised, and adopted automatically. | The continuation point is saved before the handover; the launcher starts the new build and the objective continues in it, without duplicated actions. | A build that does not start returns to the last known good build, which shows what happened and continues the objective from its saved point. |
| J8 | **Continue.** The next cycle starts from the adopted version, until the objective is met, a budget is used up, or the Operator stops it. | Progress, spend and outcomes accumulate in one record; Pause, Step and Resume take effect at the next action or tool call, and Stop interrupts at once. | Repeated failure or lack of progress stops the objective with a report, never a loop. |

**Stage mapping.** Scenario J is Stage 11 (§6.8).

### 2.12 Scenario K: Agentique tests and improves itself (C-54)

Agentique's own repository is open. The Operator gives one broad intent and
watches; agents explore, delegate, fix and adopt in visible windows.

| Step | What happens | Observable success | Failure and recovery behaviour |
|---|---|---|---|
| K1 | **Intent.** The Operator writes one broad intent in the Conversation ("Find and fix problems in …") and starts it as an objective; it is read as exploring, with its improvements and permissions, which the Operator may change. | As it starts, its thread shows what the intent was read as and by whom, and each role's effective model, why a fallback was chosen, the credential and who is billed. | A role with neither its model nor a fallback available, or a credential Agentique may not use, is named; nothing starts half-configured. |
| K2 | **Explore.** An explorer operates a test instance of the running build through the control interface. Each action is chosen from the observation, the intent, what is not yet covered, earlier findings and recent changes, among the actions that are valid there; fast typed decisions choose where they can and escalate when unsure. | The test instance's window shows the typing, clicks, focus and scrolling, the agent, its goal, each decision and its outcome, at the speed the Operator set; Pause, Step and Resume take effect between characters, Stop at once. | A crash, a hang, an unexpected dialog or a refused stale action is recovered from (restart from the same start, cancel by rule, observe again) and recorded; it never ends the run silently. |
| K3 | **Reproduce.** A finding is a deterministic check that failed: an invariant of the application or an expectation stated before the action. It is replayed from a fresh start of the same build and reduced to the steps that still reproduce it. | Each finding shows its check, its steps, the build and whether it reproduced; only reproduced findings go on. | A finding that does not reproduce is kept as such and proposes nothing. |
| K4 | **Delegate.** The lead may delegate a child objective (for example, to explore one area further) with its `delegate` tool; the directive streams into the objective's thread in the Conversation. | The Operator sees who asked whom, the child's budget and permissions within the parent's, its progress and its result returning to the parent, and can steer or stop it there. | A child that would exceed the parent's permissions, budget or depth is refused with the reason. |
| K5 | **Propose.** The lead chooses one reproduced finding and proposes an improvement; the finding's replay is among its frozen criteria. | The replay fails on the original build before any change is made. | A proposal whose regression does not fail on the original implementation is refused as no evidence. |
| K6 | **Implement, check, evaluate, review.** As J3–J5; a user-facing change is verified in a test instance. | Every criterion has an explicit outcome on the change, the replay among them. | As J3–J5. |
| K7 | **Merge, build, try, adopt.** As J6–J7. | As J6–J7. | As J6–J7. |
| K8 | **Continue informed.** Without another message from the Operator, the next exploration runs in the adopted build: it first replays the fixed finding, then prefers behaviour not yet covered and areas just changed. | The record shows coverage growing across runs, the fixed finding passing, and what each run cost. | A fixed finding that fails again is reported as a regression. |

**Stage mapping.** Scenario K is Stage 12 (§6.9).

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
- **The toolkit** is GPUI, the Operator's choice (C-48); S4.1's blind
  comparison was not run.
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
- **Panels:** Outline; Inspector (label and value rows, inline validation;
  definition, usages, overrides and specialisations of a building block);
  Requirements; Problems; History (checkpoints and "what changed"); Library
  (search with marked matches, scope and kind filters, result rows, a
  structural preview, a drag ghost; C-49).
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
display, so the CPU and GPU times are checked too. Since C-48 the Studio draws
through GPUI, which exposes no GPU timestamps: the harness reports frame
intervals and the UI's CPU time per frame, and the GPU pass is no longer
measured.

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
| Library search | ≤ 8 ms per keystroke at 5,000 blocks (CPU) | 2.4 ms (release, W5.13) | CI test ceiling in a release build (`agq-library`) |
| Memory | ≤ 300 MB private at Scenario A size; ≤ 450 MB at 10k; no growth over a 30-minute soak *(provisional until R-44)* | 351–353 MB private (322–325 MB working set) on the start screen and with URL shortener projects; 508–525 MB private at 10k | Harness reports peak private bytes; soak run at stage ends |

Feedback within 100 ms and a visible status for anything that takes more than
about 400 ms follow common responsiveness guidance [67].

**Where the budgets are enforced.**

- **CI (every pull request):** CPU-side work only, because the CI runner has no
  GPU: System State apply at 2k and 10k elements, scene update at 1k and 10k,
  layout and routing, label layout, Library search at 5,000 blocks. Ceilings are set at about twice the target
  so noise does not fail builds, and tightened as the numbers settle. A budget
  not met yet (the full scene build when a 10k project opens: 1.9 s after
  S5.1, against 1 s) has an interim ceiling at about three times today's
  reference measurement, guarding against regressions; the budget itself is
  unchanged.
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
  (S4.1, gate G6). GPUI carries AccessKit on Windows (`gpui-pre-windows`
  0.3.7); where it falls short the Operator accepts the regression (C-48), and
  the gap is listed in `docs/stages.md`.
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
- **Objectives' threads (C-54):** an objective started here from its
  intent, with what the intent was read as shown first; its thread inline,
  with the Operator's messages, directives (author → recipient, scope,
  status), results and system events told apart, each agent's entries marked
  with its role and model, tool activity and diffs folded under each step;
  replies to the thread and the objective's Pause, Step, Resume and Stop
  (§4.16).

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
| **Operator** | Intent, objectives with their budgets and permissions (C-53), high-level decisions, approving changes to locked parts, approving notes, choosing the autonomy mode, watching, steering and stopping, judging quality | Low-level implementation; routine approvals inside an objective |
| **Assistant** | Turning intent into changes to the System State through typed tools; mapping new ideas onto the architecture first; showing its plan; asking about major decisions (in the default mode); proposing notes; simulation and implementation; in a development session, working on files and commands under its permission policy (C-53) | Silently changing locked parts; changing the model outside its tools; acting outside its session's permission policy; claiming results it did not observe; remembering anything the Operator has not approved |
| **Orchestrator** (C-53) | Running objectives: cycles, their records, budgets and gates; starting the Assistant's sessions for each role; merging, building, trying and adopting when the gates pass (§4.16) | Deciding what to change (the agents); overriding a failing check; naming the locked core |
| **Language core** | Meaning and validity of the System State under the chosen KerML/SysML subset, including the built-in `Agents` library (§4.11) | UI, AI transport, persistence format |
| **System State service** | Holding the live state; applying changes atomically; enforcing locks; returning a change event for every apply, undo, redo and load | Deciding intent |
| **Library** (C-49) | Finding, describing and searching reusable definitions (the built-in blocks, the project's, My Library); working out what a block needs; planning its use, specialisation, overrides, extraction and saving as ordinary System State changes (§4.13) | Meaning and validity (the language core); applying changes and locks (the System State); drawing (the Studio) |
| **History (git)** | Durable checkpoints, branches and past states | Live editing state |
| **Simulation** (C-50) | Compiling a scenario and a fixed model snapshot into disposable runtime structures; model execution with stand-ins, replay of recordings and live evaluation through a model client it is given; traces, check verdicts, results with provenance and freshness (§4.14) | Changing the System State; calling providers or other services by itself; deciding what a block means from its name |
| **Implementation** (C-50) | Implementation links; code-to-model lookup; the implementation runner (scenarios against real code through the project's harness); the supported checks (dependency boundaries, mapped contract shapes, linked tests) and drift (§4.15) | Writing code (the Assistant, through Execution); calling a check a proof beyond its stated coverage |
| **Execution** (C-50) | Controlled side effects outside the System State: repository reads, scoped writes in a worktree, builds, tests and approved external operations, as jobs that can be cancelled and resumed; reporting the isolation the host really offers (§4.15) | Deciding what to change; claiming isolation it does not have |
| **Providers** (new) | Talking to model providers through rig; keys in the OS credential store; model lists and capabilities; usage and cost figures | Control over the System State; the Assistant's policy; being required for manual work |
| **Model providers** (Anthropic, OpenAI, OpenRouter, DeepSeek; TypeSafe AI's Jev for typed decisions only) | Language reasoning behind the Assistant and, later, behind live evaluations of agents; Jev answers typed questions for fast agents and never serves the Assistant | Anything else |

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

  The executor is to enforce "Ask before every change"; the other two differ in
  the Assistant's instructions. (Stage 6, not built yet: until then the
  Assistant works as in the default mode, and locks always ask.) What counts as a major decision is defined in the
  `decisions` skill and refined with the evaluation set (Q-3).
- **Locked parts** change only after the Operator's explicit confirmation of
  that specific change, in every mode (C-11). Inside an objective, the
  objective naming a locked element is that confirmation for the changes its
  cycles make to that element, recorded with the objective (C-53); an
  objective never names the locked core (R-16: the language core, the System
  State operations, the persistence format), which changes only by an Operator
  decision recorded in §7.6. A lock covers the part and what it owns (R-11).
  Locks are stored in the System State and versioned with it. Otherwise there
  is no "always allow" for locks.
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
  repository) never go through the model change boundary: a model edit is not
  permission to run code (C-50, Q-15). Agentique's own operations
  (verification, builds, integration) go through the Execution service. In a
  development session on the Claude Agent runtime the SDK's own file and
  command tools carry them out, inside the **permission policy** the Studio
  gives the session (C-53): where it may read and write, which paths are
  protected (the model files among them, so the model changes only through
  `apply_changes`), which commands are refused, whether it may reach the
  network, push a branch or open a pull request. Outside an objective,
  implementation work is grouped into one approval per scoped task and
  integrating a patch into the Operator's working tree asks. Locks still ask
  for every model change, unless an objective names the element. Model undo
  never pretends to undo an external action (§4.15).
- **Objectives (C-53).** An objective is the Operator's preauthorization for
  autonomous work: its intent, its budgets (cycles, attempts, exploration
  steps and model calls; spend and time when set), and the permissions it
  carries (the repository, pushing branches, opening and
  merging pull requests when the gates pass, building and adopting). Within
  it, nobody approves each step; the Orchestrator (§4.16) proceeds when the
  deterministic checks and an independent review pass, and stops when they do
  not. A locked element is changed only when the objective names it; otherwise
  the change is refused and the cycle reports it. The Operator can pause, step,
  resume, steer or stop at any time, and every action is visible.

### 4.3 The role of KerML/SysML (C-4, C-20)

- **Foundation, not scripture.** Use enough KerML/SysML that Agentique and
  everything built with it rests on a stable core, standard parts and standard
  language. Don't read the specification as gospel or chase completeness.
- **A deliberate subset, grown by need.** `docs/subset.md` lists what is
  supported, partial and excluded, each with a reason (R-8). Add a construct
  only when a scenario needs it. C-50 adds, for Scenario I: `enum def`, `enum`
  and enumeration values such as `AgentMode::fast`; the standard `dependency`
  relationship; a small expression language (literals, feature chains,
  arithmetic, comparison, logic, `if ? else`, `new T(a = …)`); the behaviour
  constructs model execution needs (`exhibit state`, states, transitions with
  `accept … via`, `accept after`, guards and effects; `send`, `assign`, `if`
  and composite actions); and verification cases as scenarios (`verification
  def` with `subject`, `objective { verify … }`, steps and `assert
  constraint`). Each is listed in §4.14 and the manifest (Q-5). C-55 adds,
  for the engineering questions of Stage 13 (§6.10), on Agentique's own model
  and on the inspection drone: referential usages (`ref part`, `ref item`), so
  a shared element is referred to instead of copied and a system can refer to
  another instance of its own kind without a composition cycle; requirement
  constraints (`assume constraint`, `require constraint`, subrequirements),
  evaluated on the modelled configuration of what satisfies them; and, where
  a calculation is reused, `calc def` with named arguments. Each is complete
  on the whole path (text, validation, System State operations, the Surface
  and the tools, execution or analysis, evidence, saving), never parser
  acceptance alone, and named in §7.6 before it is built.
- **Standard terms, standard meaning.** Deviations are recorded in one line
  each with the reason in `docs/deviations.md` (R-9), and an existing one that
  affects a scenario is corrected or its limit stated where the scenario
  meets it. Standard syntax is never given a convenient meaning of
  Agentique's own; a reference implementation accepting a model is evidence,
  not proof of conformance.
- **The same constructs for Agentique and its users** (C-20, C-55). Agentique's
  self-model uses the constructs, operations and checks it offers, and every
  extension is proven on Agentique's own model and on a system of another
  kind, so that a mechanism is general and its applications stay specific.
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
- **This phase's checks** (C-50, Q-2) are listed in §4.15, each with its
  coverage and limits: a dependency check is not a proof of runtime behaviour,
  and a file existing at a linked path is not conformance.

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

7. *(C-50)* One optional file joins the model folder: `model/links.json`,
   `{"format": 1, "repository": ..., "harness": ..., "links": [...],
   "protected": [...]}`, holding the implementation links (by element id, with
   the qualified name at the time of saving) and the implementation runner's
   binding (§4.15). History saves it atomically with the model files and
   commits it at checkpoints; a project without it opens unchanged, and older
   versions of Agentique ignore it.

The persistence format is locked core (R-16). C-50 makes exactly one additive
change to it (item 7). Settings, keys, conversations, notes, skills, run
results and job journals are stored elsewhere (§4.9).

### 4.6 Agentique designed by its own principles; the self-model first

Everything Agentique promises other systems applies to Agentique first (C-20).
The self-model in `model/Agentique.sysml` is the contract for our own
crates, checked in CI by `tools/check_architecture.py` (R-15). **Changes that
cut across parts start there**, in the same change, and the check stays green.

**Self-model changes for this phase**, in the order they land:

| When | Change to the self-model | Why |
|---|---|---|
| Stage 4 (W4.8) | Add `part def Providers` as a planned part without a crate: "Talks to model providers through rig (and a thin Jev client, C-34); keys in the OS credential store; model lists, capabilities, usage". Add `dependency from Assistant to Providers` and `dependency from Studio to Providers` (Settings tests keys and lists models) | A distinct responsibility with three users over the phase (Studio's Settings, the Assistant, live evaluations of agents), and one place that contains rig's API churn (R-21) |
| Stage 4 (W4.8) | Extend the check: the rig crates, `tokio`, `reqwest` and the credential-store crates (`keyring`, `keyring-core`, `windows-native-keyring-store`) may only be dependencies of the Providers crate; `reqwest` in `agq-assistant` is listed as a temporary exception until W5.7, as the model marks other temporary dependencies | Provider neutrality and network isolation are checked, not hoped for (§8.7) |
| Stage 5 (W5.7) | Map `part 'agq-providers' : Crate;` into `Providers`; remove `reqwest` from `agq-assistant` | The crate exists |
| Stage 6 (W6.10) | Model the Assistant as the first agent: `part def Assistant :> Agents::Agent`, with ports for its tools towards the System State and requirements for its guardrails (locks need confirmation; output is untrusted; every change is visible and undoable) and no `fallback` part: when the Assistant fails, the Operator carries on by hand, which a requirement states (a failed provider never blocks manual work) (C-44) | Dogfooding (C-13, C-20); a real example of the concept |
| Stage 6 (W6.10) | Add the standard `dependency` relationship to the subset (a recorded locked-core decision) so that CI can validate the self-model with `agq-language`; `check_architecture.py` ignores constructs it does not need instead of rejecting them | Our own language core checks our own model (R-41) |
| Stage 5 (W5.13) | Add `part def Library` with the crate `agq-library`: "finds, describes and searches reusable definitions and plans their use as System State changes" (C-49). Add `dependency from Library to SystemState`, `from Library to LanguageCore`, `from Studio to Library` and `from Assistant to Library`; the check keeps UI and network libraries out of it | One service used alike by the Surface, the palette and the Assistant; it is neither the System State's job (the live state and its changes) nor the Assistant's (the AI turn loop) |
| Stages 7–8 (C-50) | `Simulation` gets the crate `agq-simulation`, depending on LanguageCore only: live evaluation reaches a provider through a model client the Studio gives it, so Simulation never depends on Providers or the network (Q-20) | Runs stay isolated and offline by construction |
| Stages 7–8 (C-50) | `ImplementationLinks` becomes `Implementation` with the crate `agq-implementation` (links, the implementation runner, checks), depending on LanguageCore, Simulation and Execution | One part owns what the model says about code |
| Stages 7–8 (C-50) | New part `Execution` with the crate `agq-execution`: controlled side effects (repository reads, scoped writes in worktrees, builds, tests, jobs); it depends on no other part | A model edit is not permission to run code; one place decides what may run |
| Stages 7–8 (C-50) | Studio → Simulation, Implementation, Execution; Assistant → Simulation, Implementation, Execution (typed requests only: the Studio carries them out, as it applies changes) | The Surface and the Assistant use the same services |
| Stages 7–8 (C-50, moved from W6.10) | `dependency` in the subset; `tools/check_architecture.py` tolerates model constructs it does not need; the crate-dependency check also runs as an implementation check of `agq-implementation` on this repository (dogfood) | R-41; Scenario I |
| Stage 10 (C-51, W10.2) | The model moves from `models/agentique/` to `model/` at the repository root, so the repository is an ordinary Agentique project with its identity file and `model/links.json`; `models/` keeps the examples | "Develop Agentique" opens the real self-model, not a copy (one truth, §8.2) |
| Stage 10 (C-51, W10.2) | Each part states its purpose, the information it owns, its contract (ports and the items they carry) and what it must not do; the interactions between parts are connections; the three workflows of §2.8 C2 are scenarios with behaviour on the parts, and their failure paths; implementation links to crates, modules, functions and tests | The self-model explains a working product instead of listing crates (§5.6 item 6) |
| Stage 10 (C-51, W10.3) | New part `ClaudeAgentRuntime`, implemented by the TypeScript companion in `claude-agent/` (not a crate): runs the Claude Agent SDK's loop for the Assistant's SDK-backed sessions and reaches Agentique only through the tools the Studio gives it. The Assistant starts it and talks to it over standard input and output | A distinct responsibility with its own language, versions and isolation; the exception to C-34 and R-21 is recorded, not spread (§4.7) |
| Stage 10 (C-51, W10.5) | New part `Launcher` with the crate `agq-launcher`: the installed builds, which one starts, the last known good one, starting it and falling back; depends on no other part, so it works when a new build does not. Add `dependency from Studio to Launcher` | A bad self-produced build must not destroy what is needed to fix it |
| Stage 11 (C-53, W11.1) | New part `Orchestrator` (locked; its crate `agq-orchestrator` is added to the model with the crate), with `dependency from Studio to Orchestrator` and from Orchestrator to Assistant, Execution, Implementation, Providers and Launcher; the Studio gains the port `control` (the control interface) and the Orchestrator `objectives`; the requirement `GatesDecide`; the contracts of `ClaudeAgentRuntime` (the SDK's tools under the permission policy), `Assistant` (model changes only through its tools, other side effects only inside the policy), `Studio` and `Launcher` (supervising) restated | One part runs objectives, testable without a window; the gates are a safeguard, so they are locked |
| Stage 13 (C-55, W13.1) | The requirement `purpose` (definition `Purpose`, subject `Agentique`), referring to §1.1 and locked; its obligations as subrequirements follow with W13.3's constraints | The purpose is held in the model it governs, as a requirement, and protected by the existing lock |
| Stage 13 (C-55, W13.4) | The Orchestrator becomes its deterministic `Driver` (one objective's cycles as a state machine whose 34 transitions follow `run.rs`: plan and explore, replay, propose, implement, check with the evidence on the base and the gates, review, merge, build and try, adopt; repair within the attempt budget and an end to rounds without fewer failures; the next cycle while cycles are left; a child that explores once and returns) and its role agents (`Explorer`, `Lead`, `Implementer`, `Reviewer`, specialising `Agents::Agent`, one `RolePort` for every hand-off, each answering its own `RoleResult`), all nested in the locked part so its lock covers them; the lead observes the running Studio (`observe` to `studio.control`), and `connect orchestrator.control to studio.control` (driving its own Studio) is removed; its jobs, launches and replays are answered at its boundary, since a port with several connections reaches all of them in model execution; eleven scenarios for the lifecycle's success and failure paths, with 19 of the 34 transitions linked to the tests that exercise them; the test instance as `ref part testInstance : Agentique[0..1]`, another Agentique referred to, not contained. The Studio keeps one change path and one lock question for every actor (`requester`), where it had a copy for the Assistant | The autonomous lifecycle was prose only, and the Studio's model duplicated a mechanism its code has once. The scenarios check the model's account of the lifecycle; the linked tests check the code |

**Allowed dependencies after this phase** (arrows point inward; every edge
listed; not transitive):

```text
Studio → SystemState, Studio → Assistant, Studio → LanguageCore, Studio → Providers, Studio → Library
Assistant → SystemState, Assistant → LanguageCore, Assistant → Providers, Assistant → Library
Studio → Simulation, Studio → Implementation, Studio → Execution, Studio → Launcher
Assistant → Simulation, Assistant → Implementation, Assistant → Execution
Library → SystemState, Library → LanguageCore
SystemState → LanguageCore, SystemState → History
Simulation → LanguageCore
Implementation → LanguageCore, Implementation → Simulation, Implementation → Execution
Studio → Orchestrator (C-53)
Orchestrator → Assistant, Orchestrator → Execution, Orchestrator → Implementation,
Orchestrator → Providers, Orchestrator → Launcher (C-53)
Execution → (nothing in Agentique)
Providers → (nothing in Agentique)
Launcher → (nothing in Agentique)
ClaudeAgentRuntime → (no crate; reaches the System State only through the Assistant's tools)
LanguageCore → (nothing). No UI, network, async or AI types in LanguageCore,
SystemState or History.
```

**Locked core** (R-16): LanguageCore, the System State operations and the
persistence format change only with an explicit, recorded Operator decision.
This phase needs three such decisions, all in Stage 6: adding `enum def` and
enumeration values to the subset; adding the built-in `Agents` library (C-42
approves the approach; the exact text is confirmed at W6.9); and adding the
standard `dependency` relationship to the subset, so that our own core can
validate our own model (R-41). C-49 (Stage 5) adds a smaller one: the language
core exposes its existing lookup and port rule as a read-only query
(`Semantics`), so the Library keeps no second copy of a rule; no construct,
rule or meaning changes (§4.13). C-50 (Stages 7–8) authorises the minimum
additive extensions Scenario I needs, each named in §7.6 before it is built:
the three above (brought forward from Stage 6), expressions, the behaviour
subset, verification cases as scenarios, the built-in `Scenarios` library, the
System State properties that edit them, and the optional `model/links.json`
(§4.5). Existing constructs, identities, meanings and projects are unchanged.

### 4.7 Where rig sits and what it may touch (C-34)

rig (package `rig-core`, with `rig-agent` and the `rig` facade) is the
provider layer for **every** provider, Claude included (C-34). A provider that
released rig does not support, such as TypeSafe AI's Jev, gets a thin client
inside `agq-providers`, behind the same boundary and with the same small types,
until rig releases it (C-34, clarified 2026-09-27). Agentique is on 0.43.0
(released 2026-09-30, C-52 step U); 0.42.0 was released on 2026-08-17;
releases are breaking 0.x versions every few weeks, and `main` carried 20
breaking commits within two days of 0.43.0 [1][4]. That churn shapes where
rig may go. `rig-typesafeai` 0.43.0 was evaluated for Jev and not adopted
(§7.6, 2026-10-03): the thin client stays, as compatibility code where
upstream lacks what Agentique requires.

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

**The Claude Agent runtime: an SDK-owned loop (C-51, Stage 10).** One
recorded exception to "our turn loop stays ours" and to rig for every
provider, for one runtime only. The Assistant has two **runtimes** behind one
small boundary in `agq-assistant` (§4.10): the existing loop, which calls a
provider through `agq-providers` for each model call, and the **Claude Agent
runtime**, in which the official Claude Agent SDK [106] owns the reasoning and
tool iteration and its context management. Agentique keeps the project, the
tools, permissions, approvals, model changes, execution policy, review and
adoption. The SDK agent never runs inside the existing loop, and the existing
loop is not rebuilt around the SDK.

- **The companion.** A small TypeScript process (`claude-agent/`, part
  `ClaudeAgentRuntime`) runs `@anthropic-ai/claude-agent-sdk`, pinned exactly
  with its lock file, together with the Claude Code binary of that SDK version
  (whose checksum the SDK's manifest gives). The Studio starts it with Node.js
  (22.6 or later, for type stripping) and talks to it over standard input and
  output, one JSON object per line, in a versioned protocol (protocol 2 since C-53); no
  port is opened. The Studio stays in Rust and GPUI.
- **The SDK's tools under Agentique's policy, and Agentique's tools beside
  them (C-53).** A session is a complete development environment: the SDK's
  own tools (file search, read and edit, commands, subagents, skills, web
  fetch, task lists, background commands) stay on, and Agentique's tool
  definitions (§4.10) are served beside them by one in-process MCP server
  inside the companion, whose handlers only forward each call back to the
  Studio; the Studio checks the input against its schema and carries it out on
  the same path as every other tool call, model changes through System State
  operations. What the session may do is one **permission policy** the Studio
  sends at its start (the folders it may read and write, the protected paths,
  refused commands, network, pushing and pull requests, extra MCP servers),
  enforced in the companion's one pre-tool hook (the one place,
  `claude-agent/src/policy.ts`): the hook allows what the policy allows, so
  normal work never waits for a person; denies what it forbids, with the
  reason the agent reads; and sends anything else to the Studio, which decides
  from the objective's permissions or asks the Operator. Writing a model file
  with a file tool is always refused, so the model changes only through
  `apply_changes`. The project's agent configuration (`.claude/`,
  `CLAUDE.md`, `AGENTS.md`) is protected too, unless the objective names it, since it steers later
  sessions and its hooks run outside the policy; a cycle changes it only when
  its objective names it. The file tools are held to the write folders;
  commands are not confined (there is no sandbox), so the refused commands
  catch the known dangerous forms, and before a cycle's change is merged the
  Orchestrator checks every path it touches (§4.16). A project without a
  repository gets a policy with nothing to read or write, which leaves
  Agentique's tools only, as before.
- **The project's configuration, nothing of the machine's.** The project's
  own instructions (`CLAUDE.md`, which imports `AGENTS.md`), skills, hooks,
  subagent definitions and MCP servers in its `.claude/` folder are loaded
  (`settingSources: ["project"]`); the machine's user settings, auto memory
  and plugins are not, and the SDK keeps its own configuration directory under
  the app data (so its session files are Agentique's). The session's
  environment is the Studio's minus every variable that looks like a key,
  token or secret, plus the one key for its model access and the documented
  switches [107] that turn off auto memory and nonessential traffic. Managed
  policy settings a machine administrator sets cannot be turned off by an
  application; the health check says so if it finds them.
- **Model access.** An Anthropic API key, from Settings (the Credential
  Manager) or `ANTHROPIC_API_KEY`; or an Anthropic-compatible endpoint of a
  configured provider (DeepSeek documents one, `https://api.deepseek.com/anthropic`,
  for its `deepseek-v4-pro` and `deepseek-flash` models [105]), with that provider's
  key (C-53). The key is put into the companion's environment and nowhere else
  (an exception to §8.7 rule 4, recorded in §7.6), and it is the key of the
  service the endpoint belongs to. The SDK removes it from the environment of
  the session's commands, hooks and MCP servers
  (`CLAUDE_CODE_SUBPROCESS_ENV_SCRUB`; measured on 2026-10-03: without it a
  command could read the key, with it the key was absent); a process with the
  Operator's rights could still read it from the runtime's memory (there is
  no sandbox), and the Orchestrator refuses to commit or push a change that
  contains a configured key. Agentique costs a session from its usage
  at the model's own prices, never from the SDK's estimate, which assumes
  Claude's. A claude.ai login is never offered: Anthropic does not allow
  third-party products to offer it [106].
- **Sessions, steering and context.** Sessions are kept and resumed by id
  (each turn forks the session it continues); the Operator's or the
  Orchestrator's messages can be queued into a running session; the SDK's
  compaction, subagent tasks and background commands are reported to the
  Studio as they happen; Pause holds the session at its next tool call and
  Stop interrupts it.
- **Isolation stated as it is.** These controls decide what the agent may do;
  they are not an operating-system sandbox. The companion and its binary run
  with the Operator's rights and reach the model's API.

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
| Tool arguments streamed as partial JSON | no since rig 0.43 (whole when the call closes) | no since rig 0.43 | no since rig 0.43 | no since rig 0.43; on the wire, the first chunk of a call carries its id and name, later chunks its arguments [105] | not applicable |
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
| Anthropic-compatible endpoint for the Claude Agent runtime (C-53) | yes (Anthropic's own API) | no | not verified | yes: `https://api.deepseek.com/anthropic` with tools, streaming and thinking; `cache_control` and Anthropic's server tools (web search) ignored [105] | no |
| Typed decisions for an agent (C-52) | no | no | no | no | yes, for the known pinned versions in the capability table (`jev-1.13.0`); an alias such as `jev-latest` or an unknown version is not admitted |

Other providers rig supports (Gemini, Ollama and local OpenAI-compatible
servers, Mistral, xAI, Groq and more [2]) wait for a scenario need (Q-12).

Since rig 0.43.0 (C-52 step U), verified by `agq-providers`' canned-provider
tests and a probe against local streams: a tool call's input arrives whole when
the call closes, on every provider (rig buffers the fragments), so the row
"Tool arguments streamed as partial JSON" reads **no** for all four Assistant
providers and the tool card shows "Preparing change…" until then; reasoning is
sealed to the provider that wrote it and sent back only to it; usage counts
cache reads and writes inside the input, which `agq-providers` takes apart
again; Anthropic's `fallback` block still ends rig's stream (the adapter
stays); a tool call whose input is not JSON ends rig's stream, so for
Anthropic the adapter hands it to rig as a JSON string and the reply goes on,
while on the chat-completions and Responses wires the reply ends at that call
(the call is kept for an error result; what follows it and the usage are
lost).

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
| Typed decisions (C-52) | An agent whose model chats is evaluated through the answer template. An agent whose model no table knows, or whose answer a typed choice cannot fill, is refused before consent with the reason; it is never answered by another model |

### 4.9 Where settings and secrets are stored

| What | Where | Format | Owner |
|---|---|---|---|
| Settings (Operator choices) | `%APPDATA%\Agentique\settings.json` | JSON with `"format": 1`, holding only changed values; watched, so hand edits apply live; never a secret | Studio |
| API keys | Windows Credential Manager: a generic credential per provider, target `agentique:<provider>`, persistence Local | Written through `keyring-core` with the Windows store [46]; at most 2,560 bytes per secret [47] | Providers |
| Session (last project, cameras, layouts) | `%APPDATA%\Agentique\studio-session.json` (exists) | JSON; theme, contrast and reduced motion move to settings | Studio |
| Per-project data | `%APPDATA%\Agentique\projects\<folder>-<hash>\` | `conversation.json` (format 2, R-23; today's `conversations\` files move here in W5.7), `notes.md`, `skills\<name>\SKILL.md` *(provisional for notes and skills, Q-10)* | Studio and Assistant |
| App-wide notes and skills | `%APPDATA%\Agentique\notes.md`, `%APPDATA%\Agentique\skills\<name>\SKILL.md` | Markdown; one note per `- ` line; skills in the Agent Skills format [21] | Assistant |
| My Library (the Operator's building blocks, C-49) | `%APPDATA%\Agentique\library\My Library.sysml` (beside the session file) | SysML text under the package `Library`, read with the language core, written atomically; no project refers to it, since using a block copies it (§4.13) | Library |
| The model | `<project>/model/` in git | Locked persistence format (§4.5) | History |
| Implementation links and the harness binding (C-50) | `<project>/model/links.json` in git | JSON with `"format": 1` (§4.5 item 7, §4.15) | Implementation, saved through History |
| Run results and traces (C-50) | `%APPDATA%\Agentique\projects\<folder>-<hash>\runs\` | One JSON file per run with its provenance; never committed | Simulation |
| Job journals (C-50) | `%APPDATA%\Agentique\projects\<folder>-<hash>\jobs\` | One JSON file per job: states and side effects before and after | Execution |
| Kept recordings of agent answers (C-50, Q-16) | `<project>/recordings/<agent>.jsonl` | One answer per line, keyed by request digest | Simulation; committed only if the Operator chooses |
| A project's required checks (C-51) | `%APPDATA%\Agentique\projects\<folder>-<hash>\checks.json` | JSON with `"format": 1`: the commands a task of this project must pass; edited by the Operator in the Studio, never by a worker | Studio |
| The Claude Agent runtime's installed packages (C-51) | `%LOCALAPPDATA%\Agentique\runtime\claude-agent-<lock digest>\` | `node_modules` installed from `claude-agent/package-lock.json`, never changed after installing; shared by builds with the same lock file | Assistant (installed from Settings, with the Operator's approval) |
| The Claude Agent runtime's own configuration and sessions (C-51) | `claude-agent\` beside the session file | The SDK's configuration directory (`CLAUDE_CONFIG_DIR`): its session transcripts; the conversation keeps the session id | Assistant |
| Objectives (C-53) | `%APPDATA%\Agentique\objectives\<id>\` (beside the session file) | `objective.json` (`"format": 1`: intent, budgets, permissions, cycles, session ids, results, spend, continuation; written atomically) and `journal.jsonl` (one side effect per line, before and after) | Orchestrator |
| Builds of Agentique (C-51) | `%LOCALAPPDATA%\Agentique\builds\` | `builds.json` (`"format": 1`: current, history, last known good) and one folder per build with its executables, companion source and `build.json` manifest | Launcher (registry), Studio (building, trying, adopting) |
| A handover to another build (C-53) | `handover.json` in the builds folder | JSON with `"format": 1`: the build to start and its arguments (`--adopted`, the session, the project); written by a supervised Studio before it exits with the handover code, read and removed once by the launcher | Studio (writes), Launcher (reads) |
| The control endpoint (C-53) | the file `--control <file>` names (a test instance's own app data) | JSON: port, token, process, instance, version; written atomically when the Studio starts with `--control`, removed when it ends; owner-only on Unix | Studio |

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

**Runtimes (C-51).** A conversation, or an implementation task's worker, runs
on one **runtime**: the existing loop below, or the Claude Agent runtime
(§4.7), whose loop belongs to the SDK. Both sit behind one small boundary,
`Runtime` in `agq-assistant`, shaped like one turn: it takes the conversation,
the tool definitions, a bound on tool calls, the Studio's executor for tool
calls, an event sink and the stop flag. The Studio, its tool executor,
approvals, locks and conversation format stay the same for both; the input
check against the schema happens in the executor, so no runtime can skip it.
Tool calls are carried out one at a time, each bound to the project,
conversation or task that started the runtime; a call for another binding is
refused, never redirected to whatever project is open. Capabilities differ
and are shown, not hidden: the Claude Agent runtime speaks Anthropic's API
(Anthropic, or an Anthropic-compatible endpoint such as DeepSeek's, C-53),
keeps its own context and compaction (the SDK's), carries the full
development tools under the session's permission policy (§4.7), and its
session is resumed by the session id the conversation keeps; switching runtimes, or a session that
cannot be resumed, is an explicit handoff (the visible history is given as
text; nothing hidden is claimed to carry over). The loop below keeps every
other provider.

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
   `apply_changes` first shows its preview and waits for Allow. Locks always ask
   (inside an objective, unless it names the element, C-53).
5. Add the results and any queued Operator messages (R-32) as one entry; repeat
   until the model finishes, the Operator stops or sends, a refusal, a cut-off,
   an error, or the 40-call pause (kept from Stage 3; OpenAI's Agents SDK stops
   at 10 turns and VS Code asks after 50 requests [34][102]).
6. After the turn: a turn summary (changes, problems before and after, tokens,
   estimated cost); compaction if the conversation is long (R-33).

**Tools.** Five exist; three are added in Stage 6, four Library tools in
Stage 5 (C-49), the run and implementation tools in Stages 7–8 (C-50) and the
coding tools in Stage 10 (C-51). None but `apply_changes` and
`use_library_block` changes the System State (scenarios and behaviour are
written with `apply_changes`, §7.6 2026-10-01); `save_to_library` changes only
My Library, after the Operator confirms; `run_scenario` and the
implementation tools return typed requests that the Studio carries out
through Simulation, Implementation and Execution.

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
| `search_library` | Find building blocks by words, kind, scope and fit with a port; concise results (§4.13) | Stage 5 (W5.13) |
| `read_library_block` | One block in words: purpose, identity, ports and what they carry, parts, connections, settings, requirements, what it needs, whether the project has it | Stage 5 (W5.13) |
| `use_library_block` | One change: copy what the block needs into the project, add a usage (with values and an optional connection) | Stage 5 (W5.13) |
| `save_to_library` | Save a project definition to My Library, only when the Operator asked, after a visible confirmation | Stage 5 (W5.13) |
| `inspect_behaviour` | A part's effective behaviour in words (states, transitions, the configuration values its usage sees) and what, if anything, stops it from running | Stages 7–8 (C-50) |
| `list_scenarios` | The scenarios that exercise an element or the whole model, the execution modes each can run in, and their latest results with freshness | Stages 7–8 (C-50) |
| `run_scenario` / `stop_run` | One run API for model execution, replay, walkthrough and implementation; a live evaluation is the Operator's to start (refused by the schema) | Stages 7–8 (C-50) |
| `read_run` | A run's summary and checks, or a filtered segment of its trace, with stable references to events | Stages 7–8 (C-50) |
| `read_code_links` | Implementation links of an element or a file, both ways, with drift and the checks that cover them | Stages 7–8 (C-50) |
| `check_implementation` | Run the implementation checks of an element or the whole project (needs trusted-local execution) | Stages 7–8 (C-50) |
| `propose_implementation` | Propose an implementation task (outcome, affected parts, protected boundaries, permitted writes, required checks, acceptance criteria) for the Operator's approval | Stages 7–8 (C-50); the task definition in Stage 10 (C-51) |
| `observe_app`, `act_in_app` | The control interface (§4.16): what the application shows now, as text, and one visible action through the Studio's own handlers (a command, input on a control, a selection, opening a project, a wait), refused as stale when observed against another instance, project, build or screen | Stage 11 (C-53) |
| `list_files`, `read_code`, `write_code`, `run_checks`, `link_code`, `request_contract_change`, `finish_implementation` | Only inside an approved implementation task: read and write within its scope in its worktree, run its required checks, link what it wrote, return a contract change to the Operator, and report the result | Stages 7–8 (C-50) |
| `search_code`, ranged `read_code`, `edit_code`, `run_program` | Only inside an approved task: search the worktree, read a line range, replace one exact passage, and run one allowed program (a build, a test filter, a lint) for diagnostics | Stage 10 (C-51) |

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

**The built-in library** (moved from W6.9 into Stages 7–8 by C-50; recorded
under R-16 in §7.6). `AgentOutput` was added in C-50 so that the runtime reads
an answer's confidence by identity, never by a feature's name:

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
    abstract item def AgentOutput {
        doc /* An answer of an agent's model, with the model's own confidence
             * between 0 and 1. The confidence is the model's claim, not a
             * measured reliability. */
        attribute confidence : ScalarValues::Real [0..1];
    }
    abstract part def Agent {
        doc /* A part whose behaviour is produced by an AI model. Its inputs and
             * outputs are ports; the tools it may use are ports connected to the
             * parts that provide them; its guardrails are requirements whose
             * subject is the agent or its contract. model is the provider's
             * model id. When the model fails (no answer within maxLatencyMs, an
             * answer that breaks the contract, a refusal, a tool it cannot
             * reach) or answers below minConfidence, the call goes to fallback:
             * a deterministic part that shares the agent's contract. Without a
             * fallback the failure stands and no outcome is guessed. */
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
| Recorded | Replays of real answers, keyed by a digest of the canonical request (model, instructions, input, schemas, settings, and the binding: provider, model, adapter and mapping, C-52) | Yes | Realistic regression checks at no cost. A missing recording stops the run with the reason `missing-recording`; it never falls through to a live call (VCR's `none` mode [89]) |
| Live evaluation (apart from simulation) | The real model, several samples | No | Quality, real latency and cost, reported as pass rates; a good run can be kept as recordings. Temperature 0 does not make a model deterministic [90], and newer Claude models accept no temperature but 1.0 [17] |

Live evaluations are not simulation: they have external effects and are not
deterministic, so they run separately and never inside a simulation run
(§5.5). Since C-50 all three run the same scenarios through one run contract,
as the modes `model`, `replay` and `live` of §4.14: a scenario whose subject is
the agent is its set of evaluation cases. A stand-in is declared in the
scenario (`Scenarios::StandIn`), a recording is chosen by the run, and a live
answer comes only from a model client the Studio hands to a live evaluation
the Operator started explicitly. The runtime enforces the contract the same way
in each: the answer must be an item of the output port's type with valid
values; a timeout, invalid output, refusal, unreachable tool or an answer below
`minConfidence` goes to the fallback, and the trace says why (Q-13 stays open:
escalation to a deliberate agent is two agents and a routing part, never a
disguised fallback).

**Implementation of agents** (Stage 8, C-46, R-40) *(provisional until Q-2)*: an
agent is ordinary code in the project's repository, using any provider SDK.
Agentique links the agent to its code, its instructions and its recordings, and
checks: the code's input and output types match the agent's ports; the fallback
path exists; guardrail tests pass on the recordings; the latest live
evaluation's pass rate meets the agent's requirement. A failing check is drift.
C-50 settles which of these this phase builds (§4.15): mapped contract shapes
(the item types of the agent's ports against the linked code types) and the
agent's scenarios run against the real code with its model client replaced by
the scenario's stand-ins, which exercises the fallback path in the code. A
pass-rate requirement (Q-14) waits: a live evaluation reports its numbers and
the Operator judges them.

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

### 4.13 The Library: reusable building blocks (C-49)

The Operator and the Assistant build with reusable definitions instead of
modelling common structure from scratch. A **building block** is not a new
kind of element: it is any reusable definition (part, port, item,
attribute, interface, connection or requirement definition). A composite
block is a part definition with its own parts and connections behind its
ports. Using a block creates a usage typed by the definition, never a copy
of its inside: inherited features stay lookup (D-1).

**Three scopes, one experience** (R-47):

| Scope | What | How it is used |
|---|---|---|
| Built-in | About 30 neutral software definitions in `crates/library/blocks/Library.sysml`: requests, responses, messages and events; request and message ports and interfaces; service, gateway, proxy, rate limiter, authenticator; store, key-value store, cache; queue, topic, worker, scheduler; retry, circuit breaker, fallback; the composites cached store, rate-limited API, asynchronous worker and event processing. Also the standard `ScalarValues` | Copied into the project on use; standard definitions are referred to |
| Project | Every definition of the open project, where it is | Used directly, without a copy |
| My Library | The Operator's own blocks, saved from any project (§4.9) | Copied into the project on use |

- **Projects stay self-contained.** Using a built-in or My Library block
  copies its dependency closure (the definitions it refers to, transitively,
  except the standard library) into the project's own `Library` package,
  under the same qualified names, in the same change as the usage. An
  identical copy already there is reused. A different definition with the
  same name is never overwritten: the Operator uses the project's, copies
  under another name, or cancels. Changing My Library or upgrading Agentique
  never changes a project.
- **What a block says about itself comes from the model.** Name, category
  (package), purpose (`doc`), kind, ports, parts, requirements and
  relationships are read from the definitions. Where a project definition
  came from is derived: the same qualified name as a built-in or My Library
  block, with the same content or changed. Nothing else is stored.
- **One rule engine.** Which blocks fit a port is decided by the language
  core's own rule (`Semantics::ports_fit`, the `incompatible-ends` rule),
  with candidates copied into a scratch copy of the project first.
- **One operation path.** The Library plans each action (use, specialise,
  override, create a block from a selection) as one ordinary System State
  change, tried on a copy first; the Surface, the palette and the Assistant
  apply it like any other change: locks ask, undo reverts it in one step,
  history describes it in words.
- **Customising safely.** Changing a definition shared by usages names them
  first (the shared-definition confirmation, extended). A variant is a
  specialisation (`:>`), created beside the Operator's own definitions,
  never inside the copied `Library` package; a local change is a
  redefinition (`:>>`) inside the usage. The Studio calls these "Edit
  definition", "Specialise" and "Override here" and never shows the syntax.
- **The Assistant uses the same service** through four tools (§4.10). Its
  modelling skill asks, before inventing a common concept: does the project
  have it; does a Library block fit; should one be specialised; or is the
  concept different? It reuses only when the meaning fits.
- **Not in scope:** a marketplace, cloud sync, accounts, ratings, remote
  repositories, vendor catalogues, scripting or plug-ins. An "update from
  the Library" command waits for a need; the derived origin makes it
  possible (show the difference, apply deliberately, never in the
  background).
- **The `Agents` library** (§4.11) joins the same browser when it arrives in
  Stage 6, as standard definitions that are referred to, not copied.
- **Behaviour and scenarios travel with blocks** (C-50). A block's behaviour
  is part of its definition, so a usage runs it with the usage's own values.
  The dependency closure follows the references inside expressions and
  behaviour, and saving a block to My Library also saves the scenarios whose
  subject is the block. Nothing else of the project comes along.

### 4.14 Scenarios, runs and results (C-50)

**A scenario is a verification case.** It is written as a standard
`verification def` (SysML 7.22): `subject` names the system or subsystem under
examination; `objective { verify r; }` names the requirements it gives evidence
for; its steps, in order, send inputs through the subject's ports (`send new
ShortenRequest(longUrl = "…") via service.api.shorten;`), wait (`accept after
500;`), and accept outputs (`accept link : ShortLink via service.api.shorten;`);
its `assert constraint` members are its **checks**, evaluated where they stand
in the steps. Relevant failures and environment assumptions are **stand-ins**:
usages of the built-in `Scenarios::StandIn` that answer calls to one part of the
subject (an agent's model, or any dependency) with an outcome (`answer`,
`timeout`, `invalidOutput`, `refusal`, `toolUnavailable`), an optional output
item, a latency and the call it applies to. A scenario says what happens, not
where it runs: the same scenario runs in every mode below. The Operator authors
it with structured controls and the Assistant with `write_scenario`; neither
writes SysML.

**Behaviour is explicit.** A part runs only the behaviour its definition (or a
redefinition in its usage) declares: one `exhibit state` machine with an entry
transition, states with entry and exit actions, and transitions with an `accept`
trigger on a port (or `accept after` a duration), a guard and an effect.
Effects and actions are `send … via port`, `assign feature := value`, `if`
and composite actions run in the order written. Values come from the usage's
effective configuration: a redefinition in the usage wins over the definition.
Nothing is derived from a name, an icon or documentation: a part named `Queue`
has no queue behaviour unless its definition says so. An agent's output comes
from its stand-in, a recording or a live model, checked against its contract
(§4.11).

**Execution modes** (the run contract's `mode`, shown on every result):

| Mode | What runs | Deterministic | May cause external effects |
|---|---|---|---|
| Model execution (`model`) | The explicit behaviour of the model snapshot, with the scenario's stand-ins | Yes | No |
| Recorded replay (`replay`) | The model, with agents answered from recordings matched by the digest of the canonical request, binding included (C-52) | Yes | No |
| Implementation (`implementation`) | Real code through the project's harness, with the dependencies the scenario stands in for replaced by controlled stand-ins | As the code is | Only within the approved task scope (§4.15) |
| Live evaluation (`live`) | The model, with agents answered by a real provider, several samples per scenario | No | Provider calls and their cost, started explicitly by the Operator |
| Walkthrough (`walkthrough`) | Nothing runs: the steps are shown in order for explanation | — | No; never counted as verification |

**Runner contract.** A runner declares its capabilities (which step kinds and
check kinds it can evaluate, whether it sees internal state), takes a run
request (scenario, model snapshot, mode, options, limits), can be cancelled, and
produces observations and a result. A runner that cannot evaluate a scenario or
a check says so (`unsupported`); it never approximates success. Specialised
runners add their own data (a model trace, a process log).

**Model execution semantics** (the smallest set Scenario I needs; recorded in
`docs/subset.md`):

- **Snapshot and isolation.** A run compiles the model as it was when the run
  started into disposable runtime structures (instances per part usage, port
  routes from connections and interfaces, state machines, attribute slots). Its
  state is its own; editing the model during a run changes nothing under it.
- **Logical time** in milliseconds, separate from wall-clock time: a simulated
  timeout is not a measured latency. Delivering a message takes no logical time.
- **Ordering.** Events are processed in order of logical time, then of the
  order they were scheduled. A state machine handles one event at a time to
  completion. When two transitions are enabled for one event the run stops with
  `ambiguous-transition`; a message no transition accepts stops it with
  `unhandled-message`, at the element concerned.
- **Randomness.** None in this subset; the seed field of the run request is
  recorded for later stochastic workloads.
- **Termination.** A run completes when its steps are done and no events are
  pending. It stops early on a limit (events, logical time, completion depth,
  wall-clock time) or cancellation, or with an explicit reason:
  `missing-behaviour`, `missing-stand-in`, `missing-recording`,
  `agent-failed-without-fallback`, `awaited-output-missing`,
  `evaluation-error`, `unsupported`.
- **Traces** record, per event, the logical time, the element, what happened
  (message sent or received, state entered, value assigned, timer, agent answer
  and its source, fallback and why, check), the inputs and outputs, and why the
  run stopped. They are processed away from drawing and stored with the result.

**What a result may claim** (C-50, extended by C-55). These claims stay
apart, each shown for what it is and never merged into a stronger one:

| Claim | Established by | Not established by it |
|---|---|---|
| *The text parses* | The parser | That its names resolve |
| *References resolve* | Linking | That the model means what was intended |
| *The implemented semantic rules hold* | Validation under the subset | Every rule of the standard, or any physical law |
| *Calculated from the model* | Evaluating a requirement's assumed and required constraints on the modelled configuration of what satisfies it (C-55): holds, violated, assumptions not met, or not evaluable | That a built system has those values |
| *The behaviour is executable* | The runner compiled it | That it ran |
| *The run completed* | The run ended normally | That any check passed |
| *A check passed* | An assert held in that run | Anything outside that run's stand-ins and inputs |
| *The implementation agrees within the checked scope* | The checks of §4.15 | What those checks do not cover |
| *Evidence supports a requirement under stated assumptions* | Current calculations, passing scenarios and agreeing implementation checks for that requirement, with the assumptions they rest on | Validity outside those assumptions |

A `satisfy` declaration is a claim someone made, not evidence; an empty or
informal requirement, a verification case without steps or checks, a
successful parse and a scripted stand-in are never shown as stronger than
they are. Checks are shown as **passed, failed, not run, unsupported, blocked
or inconclusive**. A requirement linked to a part is not a tested
requirement; a requirement shows, kept apart, who declared it satisfied, what
the calculation on the model says, the scenarios that verify it with their
current verdicts, and the implementation checks linked to it.
Probabilistic results (live evaluation) report samples, outcomes, failure
categories and a 95% Wilson interval, and say whether each check is
deterministic or judged.

**Provenance and freshness.** Every result records the scenario, mode and
runner version, a digest of the model slice it depended on (the subject's
definitions and everything they reference, the scenario and its stand-ins),
the implementation commit and tree state for implementation runs, and the
provider, model, instructions and case versions for live evaluations.
Freshness is computed, never stored: when any recorded digest differs from the
current one the result is **outdated**, and the Studio shows it as such. A
calculation on the model (C-55) is worked out from the current model when it
is shown, with the digest of the slice it read, so it is never outdated, and
it says which values and assumptions it used. An
outdated result that belongs to an older model revision opens with that
revision's snapshot, never overlaid on today's geometry.

**Where things live.** Scenarios and stand-ins are model elements (versioned
with the model). Recordings the Operator keeps are fixtures in the project folder
(`recordings/`, committed or not as the Operator chooses; Q-16). Run results,
traces and job journals are machine-generated observations in the app's
per-project data (§4.9) and are never committed (§8.3).

### 4.15 Implementation: links, checks, execution and the supervised loop (C-50)

**Implementation links** relate model elements to code: modules, symbols, entry
points, schemas, tests, configuration, instructions and the harness, many to
many, stored in `model/links.json` by element identity (§4.5). The Studio
navigates model to code (and opens a file in the external editor) and code to
model within the mapped files. Adopting an existing codebase means adding links
where they help; nothing requires a complete reverse-engineering step.

**Supported checks** (one fully working path: Rust with Cargo), each with its
coverage stated beside its result:

| Check | What it covers | What it does not |
|---|---|---|
| Dependency boundaries | The crate (or module) dependencies of linked code against the allowed dependencies of the model; the self-model check runs this way on Agentique itself | Runtime behaviour; dependencies hidden behind dynamic loading |
| Mapped contract shapes | Linked Rust structs and enums against the item and enum definitions they implement: fields, simple types, enum values | Semantics of the values; code paths |
| Linked tests | The tests linked to an element, run and reported per test | Anything the tests do not exercise |
| Scenarios against the implementation | The scenario's steps and checks through the harness, with stand-ins for the dependencies the scenario names | Internal state the harness does not expose (such checks are `unsupported`) |

A failing check on linked code is **drift** at the elements it covers. A file
existing at a linked path is never reported as conformance.

**The harness** is a small program in the implementation repository, linked as
the scenario entry point. The implementation runner starts it and exchanges one
JSON line per step: the runner sends inputs and stand-in answers, the harness
calls the real code and reports what the system sends out. The runner evaluates
the scenario's expressions and checks itself, with the same evaluator as model
execution; it never replays expected answers. Generated systems run without
Agentique.

**The Execution service** carries out every side effect outside the System
State as typed operations: read and list files in the repository, write and
delete files inside a task's worktree, run allow-listed commands (Cargo, the
project's commands, an objective's pushes and pull requests as exact commands, the
harness) with a timeout, create and remove worktrees, and integrate a reviewed
patch. Paths are canonicalised and any path escaping the scope is refused.
Commands run with a scrubbed environment (no provider keys, tokens or other
secrets) and, by default, Cargo offline. On this host there is no process,
filesystem or network sandbox, so execution runs only in an explicit
**trusted-local** mode the Operator turns on per project, and the Studio says
what that means: a worktree isolates edits, not processes, and build scripts
and tests run with the Operator's rights. Jobs have stable ids and states
(pending, running, waiting for you, done, failed, refused, cancelled,
interrupted), a journal that records each side effect before and after it
happens, cancellation that ends the whole process tree, and recovery that
never repeats a completed side effect. Nothing claims atomicity across the
model, the repository and processes: a partly completed operation is shown with
what completed and how to reconcile.

**The supervised implementation loop.** One worker (the Assistant with the
implementation tools) takes one approved task through: inspect the model and
code → identify scope → propose a plan → implement → run checks → inspect
failures → repair within scope → present the result. Its context is derived
from the model each time (the parts in scope with their contracts,
dependencies, requirements, protected boundaries, code locations, base
revisions and acceptance scenarios), never a hand-kept second specification.
It writes only in the task's worktree and scope; protected tests, accepted
requirements, scenarios and evaluation policy are outside its write scope, so
it cannot make a failing check pass by weakening it. A contract change it
thinks it needs ends the task with that request for the Operator. Repair rounds
are bounded, and the same failures twice in a row end the task with a blocker
report. The result is a runnable implementation with observed check results,
reviewed beside the affected parts and contracts before the Operator integrates
it; integration checks the base revision again and never touches the
Operator's uncommitted work.

**Tasks, required checks and integration (C-51, Stage 10).** These hold for
every project; Agentique's own repository is the first that needs all of them.

- **The task definition is fixed at approval:** the requested outcome, the
  affected parts, the protected boundaries, the source revision and model
  digest it starts from, the paths it may write, the **required checks** and
  the acceptance criteria. The required checks are the project's commands
  (kept per project in the app data, `checks.json`, edited only by the
  Operator: for a Cargo workspace a build, the tests, clippy and rustfmt; for
  Agentique also the architecture check and the companion's tests) plus the
  checks derived from the links and the brief's scenarios at that moment: the
  build; dependency boundaries when the links name modules; contract shapes
  and linked tests; the brief's scenarios when a harness is linked. What
  cannot be checked because it is not configured (no harness, no module
  links) is listed at approval as not checked, never counted as a pass. A
  worker can add checks by proposing links; it cannot remove a required one.
  Changing the required set is a separate, visible Operator decision recorded
  in the task. A task's worktree starts from the last commit, so when the
  model is in the code repository a task starts only after a checkpoint: the
  worker never works on an older model, or without a lock set since.
- **Every required check ends with an explicit outcome:** passed, failed, not
  run (with the reason: not configured, no harness, no tests found, cancelled,
  refused, did not build), unsupported, blocked or inconclusive. A
  verification passes only when every required check passed and nothing else
  failed; anything else is shown as what it is. A verification records the
  task commit and model digest it checked, and is outdated as soon as either
  differs.
- **The task commit.** When the worker finishes, the Studio commits the
  worktree on the task's branch; verification, review, builds and integration
  all refer to that commit. Model changes a task proposes are made in the
  task's worktree's copy of the model with the Assistant's own model tools
  and ordinary System State operations, never by writing model files; a
  locked element is refused (a worker cannot confirm) unless the task is an
  objective's cycle and the objective names it (C-53). The Studio verifies a
  clean checkout of the task commit (files the commit does not hold count
  for nothing) against that model, and the review shows the model changes
  (by element identity) beside the code changes. Integration checks them
  again: a task commit that changed an element locked then or now, the
  locks, or any file of the model folder other than its documents and
  identities (`links.json` holds the protected paths) is refused, however it
  was made; after integrating, the Studio reads the accepted model again.
- **Integration** is the Operator's, enabled when the verification is current
  and passed (otherwise "Integrate without a passing verification…" names
  every required check that did not pass and asks). It brings exactly the
  reviewed commit into the project's branch when the branch has not moved:
  only the task's files are written, nothing else staged or unstaged is
  touched, protected paths are refused again, and each step is journaled, so
  an interrupted integration is finished or reported, never repeated. Code
  linked to a locked part, and Agentique's safeguards (execution policy,
  verification, the Claude Agent runtime, the launcher), are locked in its
  model: a patch that changes their implementation asks explicitly at
  integration (one confirmation, recorded in the task, covers this and an
  unverified task). Outside an objective, Agentique never pushes. Inside an
  objective whose permissions allow it (C-53), the Orchestrator pushes the
  cycle's branch, opens a pull request and merges it when the gates of §4.16
  pass; it never force-pushes, never pushes to the default branch directly and
  never bypasses the repository's own rules. A safeguard changed by a cycle
  is merged only when the objective names it, and the review says so.
- **Builds, trying and adopting** (for Agentique). A build is a release build
  of one commit into its own folder with a manifest: source commit and tree,
  toolchain, the companion's lock digest, the check results it was built
  after, the executables' digests and the data formats the build reads and
  writes. "Try this build" starts it as a **test instance**: its own app data,
  Claude Agent sessions and builds folder, opened on a worktree, never on the
  project the running Studio has open. "Use this build" asks, checks that the
  executable is the one built and that its commit is the integrated one, backs
  up the app data, saves the session and hands over to the launcher, which
  starts the new build and the development project. A build that changes a
  data format cannot be adopted until rolling its data back is supported.
  Inside an objective (C-53) the Orchestrator does the same without asking,
  after the build's test instance has passed the cycle's checks, and saves the
  objective's continuation point before the handover.
- **The launcher** (`agq-launcher`) is the entry point that starts the current
  build. If a build exits or does not report ready within its time, it marks
  the build failed, writes the reason to its log and starts the last known
  good build, which shows what happened. Started to supervise (C-53), it
  stays running as the Studio's parent: when the Studio exits to
  hand over to another build, it starts that build; when the Studio crashes,
  it starts it once more and then falls back to the last known good build;
  when the Studio closes normally, it ends. A build becomes the last known
  good one only after it started and reported ready, which a build started
  for an adoption does only after its check after adoption (§4.16). `agentique-launcher --recover` (Settings › About shows the full
  command) starts the last known good build in safe mode (no Claude Agent
  runtime). It never builds, downloads or deletes anything.

### 4.16 Objectives: the Orchestrator, the control interface and typed decisions (C-53, C-54)

**The Orchestrator** (part `Orchestrator`, crate `agq-orchestrator`) runs
objectives (§4.2). It owns each objective's record in the app data
(`objectives/<id>/`): the intent, budgets, permissions, cycles, the agents'
session ids, results, spend and the continuation point, as a state file
written atomically and an append-only journal. It is deterministic code: it
decides which phase comes next from recorded results; agents decide what to
change. Every side effect is journaled before and after under a key, so
resuming never repeats one that completed.

**A cycle** takes one improvement from proposal to adoption:

| Phase | Who | Done when |
|---|---|---|
| Explore (C-54; when the objective explores) | The Orchestrator runs an explorer in a test instance of the running build (below, "Exploration"); the lead may first delegate a child objective to explore an area | The run ended within its budgets, with its coverage, findings and decisions recorded |
| Reproduce (C-54) | The Orchestrator: each new finding replayed twice from a fresh start of the same build, then reduced | Each finding is reproduced or not, with its reduced steps |
| Propose | Lead agent: inspects the self-model, the code, recent results, the reproduced findings and the running application, reading only (it runs no command); may delegate to subagents | It submits one improvement: the parts it affects, why, a plan, and acceptance criteria, each with how it is checked (a test run, `cargo test`, `node --test` or `python -m unittest`; the replay of a reproduced finding (C-54); or an observation of the running application). No criterion may pass on the base, and at least one must fail there with evidence (C-54, "Evidence" below). The criteria are then frozen for the cycle |
| Implement | Implementer agent in the cycle's worktree, with the development tools under the cycle's policy and model changes through Agentique's tools on the worktree's model | It submits the implementation; the Orchestrator makes the cycle's commit from the worktree (the agent's own local commits are kept in it) |
| Check | The Orchestrator, through Execution, on a clean checkout of that commit | Every required check (§4.15) and every criterion's test or command has an explicit outcome |
| Evaluate | The Orchestrator builds the commit (a debug build in the cycle's worktree) and starts it as a test instance; an evaluator agent operates it through the control interface, its control tools bound to that instance. It runs whenever a criterion is behavioural or the change touches user-facing code (C-54) | Each behavioural criterion has an outcome: a deterministic assertion where the criterion is precise, otherwise the evaluator's judgment with the observations it rests on; a judgment never turns a failed assertion into a pass |
| Review | Reviewer agent: a fresh session with read-only tools and no commands | Approve, or findings. It sees the frozen criteria, the check results and the diff against the base, and judges changes to tests, requirements and checks explicitly |
| Repair | The implementer, resumed in the same worktree | Back to Check, within the attempt budget; the same failure twice in a row, or rounds without fewer failures, end the cycle with a blocker report |
| Merge | The Orchestrator | One commit holding the reviewed tree is pushed (on the last one pushed, or the base, so no earlier attempt leaves this computer), a pull request opened, the repository's checks are green on exactly that commit, and it is merged; the local default branch follows and an open project of it is read again. When the repository's checks fail only in tests of crates the change neither touches nor can affect by the workspace's dependencies, and that read no files outside their own crate (W13.7), the failure is not the change's: the reviewed change stays on its pull request, the thread says what failed and why, and, where the objective may merge, a repair cycle (not one of its improvements) repairs the cause; once that is merged, the reviewed change is carried onto it only if the repair touches what failed and the change's patch is unchanged, and merged only when the repository's checks pass on it. Checks that never reached a verdict (cancelled, or failed setting the job up) are the checks' own machinery: the change waits on its pull request and nothing is repaired. Nothing reruns a failure until it happens to pass |
| Build and try | The Orchestrator | A release build of the merged commit (§4.15), executables verified against its manifest, started as a test instance with its own data and exercised through the control interface: it starts, reports ready, opens the project, and the criteria observable in the application hold |
| Adopt | The Orchestrator and the launcher | The continuation is saved, sessions and jobs are stopped, and the launcher starts the new build, which resumes the objective after checking it is the build that was adopted |

**Gates.** A cycle's implementation is a task (§4.15) whose definition is
the frozen proposal, so its commit goes through the same checks as any task's
before it is merged: protected paths, the model folder's rules, elements
locked then or now, code of locked parts and Agentique's safeguards, each
allowed only when the objective names it, and no configured key in the
change. A cycle merges only when every required check passed on the reviewed
commit, every acceptance criterion passed, one criterion showed the defect on
the base with evidence and a user-facing change was evaluated in a test
instance (C-54), the reviewer approved and the repository's own checks
passed; it adopts only when its build's test instance passed. A
required check that did not run, a criterion without an outcome or a missing
review is a failure, never a pass. A baseline guard compares the cycle's diff
with its base: deleted or ignored tests, removed assertions, relaxed budgets
and changed required checks are listed for the reviewer as test changes, and
fail the cycle unless the proposal named them as intended, with a reason the
reviewer accepted. The criteria and the required checks are frozen at the
proposal; a later attempt cannot replace them. A change to `ROADMAP.md` or to
the self-model's purpose requirement (`Purpose` and `purpose` and
everything they own, by identity) fails the gates even when the objective names it, as the locked core does
(C-55): the governing text and the purpose change only by the Operator's
decision.

**Alignment with the purpose (C-55).** Every proposal names, besides its
parts, plan and criteria: the requirements of the project's model it serves
(`serves`: the capability or root requirement, by qualified name), the model
elements and contracts it affects (`parts`), the expected user-visible
benefit, its evidence (the frozen criteria, the replay of the reproduced
finding among them when there is one), and its effect on root complexity,
reuse and dependencies (`complexity`). The
Orchestrator checks mechanically, on the base's model, that each named
requirement and element exists, and refuses the proposal otherwise. At
review it lists what the commit changed (model elements by identity, and the
parts whose linked code changed) beside what the proposal named, so changes
not named and names not changed are judged explicitly; and it summarises the
**cumulative** change from the Operator's **approved baseline** (the commit
the tag `approved-baseline` marks, which only the Operator moves) to the
reviewed commit: elements added, removed and changed, counted at the root
(part definitions, dependencies, requirements, locks), and the tests and
checks changed over that range. The Orchestrator reads the tag from the
repository's remote, not from a worktree an agent works in, and records the
commit it compared with; worktree sessions are refused tag commands, and every session the alias and remote-ref commands. The reviewer judges whether the cumulative
change still serves the purpose and the named requirements, so that small,
individually plausible changes cannot redefine the product over many cycles.
The mechanical checks say what changed; only the review judges alignment, and
neither a number nor an agent's approval proves it. Before a reproduced
finding is fixed, the lead records its **disposition** in the testing
knowledge: a genuine defect, a wrong expectation of the tester, an ambiguous
requirement (a question for the Operator, not a fix), or an unreliable
reproduction, judged against the requirements and intended semantics; only a
defect is proposed, an explorer's own expectation is never one until it is
judged so, and a finding judged a wrong expectation or unreliable is not
offered again, so repeated disagreement with a model-generated expectation
does not turn into a change.

**Budgets and progress.** An objective has a cycle budget (the improvements it
makes), an attempt budget per phase, an exploration budget in steps and a
budget of model calls per role (C-54), and may have a spend budget (USD, from
usage at the models' own prices, typed decisions included) and a time budget
(§1.6, §4.2 and C-37 say "spend, attempts, time" for short). The Operator
starts an objective from its intent: whether it explores, its cycles and
whether it may merge and adopt are inferred from the intent by a typed
decision and shown before Start, where the Operator may change them; spend
and time are unlimited unless set (a child objective's are), and the costs
of its work are counted and shown either way
(the Operator's amendment of C-54, §7.6). Before each phase the Orchestrator
checks what is left; a budget used up stops the objective with its record. Repeated
identical failures (the same check with the same failure), a proposal that
repeats a failed one, or rounds without fewer failures count as no progress
and stop the cycle with a blocker report.

**Agents and sessions.** Each role is its own Claude Agent runtime session
with its own instructions, tools, permission policy and effective model
(C-54, below), so the reviewer starts with fresh context; within its session,
the lead or the implementer may delegate to the SDK's subagents. The explorer
is not a session: the Orchestrator's exploration asks its model one typed
question per step (below). The implementer writes only
in the cycle's worktree; the reviewer and the evaluator write nothing. Session
ids are kept in the objective's record, so a restarted Studio resumes them.
The Operator's messages to an objective are queued into its running session.

**The control interface** lets agents operate the real, visible Studio without
computer vision. The Studio publishes a structured **observation**: its
identity (instance, build and commit, project, session) and revision; the
screen, view, panels, selection, the dialog's kind (its fields and buttons
are among the controls), palette, status and problems; the conversation's
state, tasks, builds and the objective; every drawn control (stable id, role,
label, value except a masked field's, enabled, selected, focused, and the
bounds of its visible part, in painting order so the topmost comes last); and
the commands with whether they are available, and why not. **Actions** reach
the handlers the Operator's input reaches: commands through the Studio's
command dispatch; click, type, key and fill through the window's own input
dispatch at the control's bounds, so focus, hit testing, dialogs and
rendering are exercised; selecting an element; waiting for a condition
(beside other actions). An action names the identity and the observation
(its screen revision) it rests on: one for another instance, project,
session or build, a screen that changed since (screen, dialog, palette,
project, settings section or selection), or a control that is gone or
disabled is refused as stale; a request nobody waits for any more is dropped.
**The Operator's own** stays the Operator's (§3 roles): an agent cannot
answer a dialog that asks for the Operator's approval (integrating a task,
changing locked elements, trusted-local execution, a paid live run, starting
an implementation), act in Settings (it may leave them), in the
Conversation or in the Objectives panel, lock or unlock, undo, change the
appearance, or pause and resume agents. One exception follows C-54: in a
**test instance** (a build the Orchestrator starts with `--test-instance`,
its own app data and no work of the Operator's), agents may operate the
Conversation (focus the composer, type, send, stop a turn) and undo and redo,
so self-testing exercises them as a person would; approvals stay refused
there too. In the Operator's own window, agents never type into the
Conversation or act as the Operator: their directives and results reach it as
their own entries ("The Conversation, one window" below). This holds where each of those effects happens, whatever
route reached it (a click, keys, focus and Space, the palette), for as long
as a step of an agent's action and the work it dispatched run; requests are
also refused up front with the reason where that can be told, and text goes
only to a focused field. The endpoint's holder supervises its instance: the
endpoint's Pause, Step and Resume are the supervisor's, not an agent's. A change an agent's step makes is recorded as the
Assistant's, with the agent named in its description, and so is what an
agent typed into a panel's field when that field commits later, whoever
ends the edit. Every action and its effect go to an event
trace. Agents reach it through Agentique's tools, bound either to the
running Studio or to a test instance; the Orchestrator reaches a test instance
through a local endpoint (127.0.0.1, a random port, a token in that instance's
app data). Screenshots are for the
Operator; an observation is text, and no agent claims to have seen pixels.
The endpoint refuses a line over 1 MB, more than eight connections, and a
connection idle longer than an answer may take, before any token is checked.

**Visible agents.** A ring and a label mark the control an agent acts on; the
title bar shows what the objective's agents are doing; the objective's
thread keeps the activity record, shown in the Conversation and the
Objectives panel. **Pause** holds the agents at their next action or
tool call, **Step** lets one through, **Resume** continues and **Stop**
interrupts every session of the objective. Observing costs one map entry per
drawn control and makes nothing wait.

**Typed decisions in operation (System 1).** Reasoning agents (System 2) plan,
implement, diagnose, review and judge. Fast typed decisions (Jev, C-35 as
amended by C-53) make bounded choices during operation: one atomic question
with explicit inputs and typed options (for example, which of the actions a
dialog offers continues a journey toward its goal without losing work),
under a deadline. A confident answer in time is used; low confidence, an
invalid answer, a timeout, an unavailable provider or a cancellation
escalates to the reasoning model; where the answer is already known,
deterministic code decides and no model is asked. A dialog asking for the
Operator's approval waits. A dialog that stands in the Orchestrator's way is
never its goal, so before each observation criterion the Orchestrator
cancels one left open by rule and names it in the criterion's outcome; only
Cancel is pressed, a build that starts with a dialog open fails, and when the
way cannot be cleared (an approval left open, or the instance gone) the
criterion is not run, which is no pass. A typed decision never overrides a
failing check, and its cost and latency (a failed call's too) are
recorded. The model a decision escalates to, and Jev's model, are the
`escalation` and `decisions` roles of Settings (C-54), which replaces the
fixed `Decider::default` of the W11.6 entry in §7.6. The first Jev-assisted application-control workflow, a dialog
in the way of a goal in a real test instance, is measured against the rules
and the reasoning model alone (W11.6, `docs/stages.md`); a typed decision is
used in operation only where no rule knows the answer.

**Lifecycle.** The launcher supervises (§4.15). Before a handover the
Orchestrator writes the continuation point (the phase reached, the adopted
build, session ids, what completed), the Studio stops the Assistant's turn,
runs, the implementation task, checks and builds and waits for them to end
(test instances the Operator started stay theirs), and exits with the
handover code; the handover names the build, the session and the project open
now; History's lock file (`model/agentique.lock`)
keeps one writer at a time. The **check after adoption**: the new build,
started for an adoption, writes its ready file only after it has confirmed
that its own manifest is the adopted build, opened the project and read the
objective's continuation; the launcher, which owns the registry, records it as
last known good when it reports ready. The new build then continues. A build
that does not start, or fails that check, returns to the last known good
build, which records the failure in the objective and continues from the
saved point. Data formats are compared
before adoption; a build that changes one is not adopted.

**Models per role (C-54).** Settings name, for each role, a provider and
model, an effort and a fallback: the Assistant, the lead, the implementer, the
reviewer, the evaluator, the explorer, escalation (the reasoning model a typed
decision escalates to) and typed decisions. The defaults follow the work:
Claude Opus 5.5 (`claude-opus-5-5`) for the lead, review and escalation;
Claude Sonnet 5.5 (`claude-sonnet-5-5`) for the implementer, the evaluator and
the Assistant; DeepSeek's `deepseek-flash` for exploration; Jev for typed
decisions; DeepSeek's models as the fallbacks. The Assistant's row is the
existing Conversation setting (`assistant.provider`, `assistant.model`,
`assistant.effort`), whose Anthropic default becomes Sonnet 5.5; it is not
duplicated. Before an objective starts, the Studio, which owns Settings and
the keys, resolves each role's effective model and hands it to the
Orchestrator, which records it and uses it for every session of that role:
its own model when that
provider has a credential Agentique may use, otherwise its fallback, with the
reason; a role with neither stops the objective from starting. No role takes
another's model by default. A role's session, effort, permission policy,
SDK subagents and spend stay that role's; the record and the Objectives
panel show the effective model, any fallback and why, and spend per role.

**Credentials.** A Claude Agent runtime session uses only a credential the
Studio gives it: for Anthropic, an API key (billed per token to that key's
account) or the Operator's own Claude subscription token, the long-lived
token the documented `claude setup-token` prints for scripts
(`CLAUDE_CODE_OAUTH_TOKEN`, counted against the Operator's plan limits) [108];
for an Anthropic-compatible endpoint, that provider's key. The Operator
chose, on 2026-10-04, to give Agentique such a token for their own work on
their own machine (C-54); Agentique never offers a claude.ai login, never
reads the Claude CLI's stored login, and never extracts or copies a
credential. A subscription token is used only by the Claude Agent runtime
(the SDK's own Claude Code binary), never in a direct API call, so roles that
call a model directly (escalation, typed decisions, the Assistant's own
loop) need an API key or take their fallback. Exactly one credential goes to
a session; its own configuration folder keeps the machine's Claude login out
of reach; the companion checks the source the SDK reports before the first
prompt (`apiKeySource` for a key, `accountInfo().tokenSource` for a token)
and stops a session that would use any other; the credential is kept out of
the session's commands, hooks and MCP servers. A local Claude login is
detected with the documented `claude auth status` and shown, with the reason
it is not used: Anthropic does not allow products built on the Agent SDK to
offer claude.ai login without its approval [106][108]. Settings show which
credential each provider uses and who is billed (the key's account at
Anthropic, DeepSeek or TypeSafe AI, or the Operator's Claude plan; Agentique
cannot see an account's organisation); spend on a subscription is shown at
API prices, as API-equivalent, and budgets apply to it. Nothing switches to
another credential or paid account silently: a usage limit stops that role's
session with the reason.

**Exploration.** An explorer's run has a test instance of one build, a goal,
the testing knowledge, budgets (steps, time, spend) and a way of deciding.
What it explores is the project its lead's plan names (a folder of the
repository that holds a model, `model` for Agentique's own) at the commit of
the build explored, with the goal and, where useful, the model elements it is
about and where to start. A plan the lead's turn accepts is recorded on the
objective at once. A child explores it, or another project the lead names
among those the objective may explore (before the first plan, the lead must
name one). Later explorations, replays and the evaluation's exploration keep
to the plan's projects, others only when the plan permitted them; a lead that
plans no project is asked once more, and then the cycle ends: nothing
alternates between projects. The instance opens a copy of the project;
before its first action an exploration or a replay checks the copy (the
folder copied, its files' digest, the project shown), and a mismatch acts on
nothing and is said in the thread. Each step: observe; check the invariants;
list the actions that are valid there (enabled, visible controls and
available commands that the
observation offers to agents, with typed inputs from fixed input classes: a
valid name, empty, long, non-ASCII, a duplicate, a keyword); choose one (by
rule, by Jev, by the explorer's model, or by Jev escalating when unsure); act
as the explorer, with the goal and the reason for observer mode; record. A
**finding** is a deterministic check that failed: an invariant (the instance
exits or stops answering; a control the observation offered to agents, enabled,
is refused as gone or disabled on a screen that did not change; an
interactive control has no readable label; undo, performed by the explorer
itself in its test instance, does not restore the model; a dialog other than
an approval cannot be closed by its Cancel or Escape; an action takes longer
than its budget; the status line reports an internal error) or an
expectation the explorer stated before acting, in the grammar of observation
criteria (screen, dialog, status, selection, and a control's label, value or
state). A refusal by rule (the Operator's own, a stale action) is never a
finding. A model's opinion alone is never a finding. A finding is
**reproduced** by replaying its steps from a fresh start of the same build
(actions re-based on fresh observations, no model asked) twice, both failing
the same way, and **reduced** by dropping steps while it still reproduces;
its identity is its check, control and message with numbers, paths and
element names normalised. A crash, a hang
or an unexpected dialog is recovered from by restarting from the same start
or cancelling by rule, and recorded.

**Testing knowledge.** The Orchestrator keeps its testing knowledge per
project with its records in the app data (`testing/`): coverage (screen,
region, control, action and input class, with counts and the build last
seen), findings (steps, state, the cycle and pull request that fixed them)
and runs (way of deciding, new coverage, findings, latency, cost). Useful
coverage is what the record had not seen before; progress is reaching the
areas a goal names. Exploration prefers what is not yet covered and the areas
changed since the last run, and first replays the findings fixed since; two
runs from the same start take materially different paths when they cover
different actions within their first steps. A change to how exploration or a
typed decision chooses is adopted only on evidence from fixed exploration
tasks held out from its tuning; the tasks that judge it are never weakened to
let it pass.

**The Conversation, one place (C-54, the Operator's clarification of
2026-10-04, "one window, role models").** The Conversation is where the
Operator gives intent, follows every agent and steers; the Objectives panel is a dashboard of the same
objectives (budgets, phases, effective models and spend per role, the tree of
child objectives, Pause, Step, Resume, Stop). Both read the same objective
records and use the same application commands (start, message, pause, step,
resume, stop). The Assistant is the Operator's entry point and answers
ordinary requests on its own role's model. Autonomous work becomes an
objective: the Operator starts it from the Conversation (or the panel) from
its intent, with what the intent was read as (whether it explores, its
improvements, whether it merges and adopts) shown first and open to change,
or the Assistant proposes one with a tool and the Operator starts it; starting stays the
Operator's own. From then on the objective's **thread** appears in the
Conversation in time order: the Operator's messages, agents' directives and
results, and system events (phases, checks, pull requests, merges, builds,
adoptions, recoveries), each marked with who wrote it (role and effective
model, or "you", or Agentique), with tool activity and diffs folded under
the step they belong to. No agent forwards messages: the Operator's reply in
an objective's thread (the same command as the panel's message field) goes to
the implementer at its next tool call while it works, and otherwise waits in
the objective's record for the lead's next turn; the reviewer, whose
judgment must stay independent, and the explorer, which is not a session,
never receive it. The thread shows where each message went. The thread is
kept with the objective's records (`objectives/<id>/thread.jsonl`), not in
the conversation file, and no model receives it whole.

**Directives and delegation.** A **directive** is a first-class record of
one agent's instruction to another: its author (role and objective), its
recipient (a role, or a child objective), its parent objective, its scope
(the instruction, and for a child its budgets and permissions), its status
(running, done, failed, stopped or refused, with the reason) and its result.
The Orchestrator's handoffs within a cycle are directives that refer to the
record they hand over (the lead's proposal to the implementer, the reviewer's
findings for repair), and so is
delegation: an objective's lead calls its `delegate` tool, and the
Orchestrator validates and records the directive and starts the **child
objective**, with a budget within what the parent has left and permissions
within the parent's (an exploration child never merges or adopts). Text in
the Conversation is never a directive by itself: only the tool, validated by
the Orchestrator, creates one, and the Conversation shows what was recorded.
The directive's text streams into the thread as it is recorded, and the
child's work and result follow it. Each session receives only its own
handoff (the directive and the result it needs), never another agent's
transcript, and no agent answers events it merely observes. A child counts
against its parent's budgets, children nest at most two deep, one runs at a
time per parent, it stops when its parent stops, and its result returns to
the parent's lead, which continues; the Operator sees, steers and stops it
like any objective. Agents cannot start, steer or stop the Operator's own
objectives, answer the Operator's approvals, act as the Operator or widen
their scope.

**Durable, continuing work.** An objective's progress lives in its records,
not in the Conversation or a model's context: closing the Conversation,
compaction of a session, an adoption, a crash recovered by the launcher, or
the Operator closing and reopening Agentique leaves it able to continue from
its saved point without repeating a completed side effect. It continues by
itself after an adoption, or a fallback to the last known good build
(`--recovered-from`), saying so in its thread; one interrupted because the
Operator closed Agentique waits for the Operator's Continue; a crash the
launcher restarts in the same build is not recognised as a restart (the
launcher passes no flag, and it is a locked part), so such an objective
waits for the Operator's Continue, as after a close; a Pause holds across
restarts; a resume that fails twice stops the objective with its record.
Within the Operator's budgets and permissions, one intent runs a continuing
loop: an exploring objective explores, fixes, adopts, and explores the
adopted build again, cycle after cycle, without waiting for another
message, until its budgets are used up, two explorations in a row reproduce
no new problem while no reproduced finding left unfixed is still eligible (a
reproduced finding a cycle did not fix is offered again, once it reproduces
on the new base, until the objective has tried it twice), no progress stops
it (as above), or the Operator stops it.

**Credentials of test instances** (decided in this session, pending the
Operator, §7.6). A test instance of a merged build used for exploration may
be given the explorer's provider key for its own Assistant, so self-testing
of the Conversation reaches real answers within the objective's spend
budget. A test instance of an unreviewed build (evaluation) receives no
credential; its Assistant runs on the scripted stand-in the journeys use, so
the Conversation's own behaviour is testable, and a criterion that needs a
real model there is not run, never a pass. A test instance can also start
with a recorded objective, so its thread can be observed and operated. A
Studio started with `--test-instance` must have its own app data (its own
`--session`, not the Operator's), reads keys only from its environment,
never from the operating system's credential store, and does not probe the
machine's Claude login; this keeps accidents out, while real isolation from
code an unreviewed build runs needs the operating system (§1.6).

**Control ownership and observer mode.** One agent acts in a window at a
time: the first to act holds it until it releases it or has been idle for 30
seconds, and another agent's action is refused with who holds it; the
Operator's own input is never refused. The supervisor (the endpoint's
holder, which drives a test instance for the Orchestrator) acts without
holding the window, and an Assistant turn an agent's message started acts
within that agent's hold. Each explorer and evaluator has its own test
instance. Observer mode extends the visible agents above with a speed the
Operator sets (the setting `control.speed`, or `--control-speed` for a test
instance): `observe` (typing about 12 characters a second, a short pause on
the target before a click), `fast` (a character a frame) or `instant` (as
before, for tests); clicks, focus, scrolling, the acting agent, its goal, its
last decision and the outcome are drawn in the window. Pause, Step and Resume
take effect between characters, and Stop at once; after the Operator's Stop,
only the Operator resumes. Every refused action carries a kind (the
Operator's own, stale, gone, disabled, unavailable, held, stopped, expired,
invalid, timeout, failed) beside its reason, so clients never read meaning
from wording. In the Operator's own window, an observation names the
Conversation's entries without their text and leaves out the composer's
draft; a test instance shows them, for self-testing.

**Evidence, gates and bounds added (C-54).** A criterion shows a defect on
the original implementation only by behaviour: a replay or observation that
fails on the base build, or a test that compiles and runs on the base (new
test files brought over) and fails there. No tests, a compile error or a
broken setup on the base is no evidence, and a cycle needs at least one
criterion with evidence. A change to user-facing code (the code of the part
`Studio`, by its links, outside tests) needs a behavioural criterion and an
evaluation in a test instance, whatever its other criteria. A judgment's
failure is identified by its criterion and verdict, not its wording.
Attempts, steps and model calls per role are budgets with defaults, and spend
and time are budgets when set (the Operator's amendment of C-54, §7.6);
a cycle's worktrees are removed when it ends (the work of a failed or
interrupted cycle is kept, the three most recent); one build runs at a time;
merged branches are deleted. A test instance can be started in a stated
condition (for example, recovered from a failed build), so criteria about
such states are testable.

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
| `crates/library` | **New** (C-49): the Library service and the built-in blocks | §4.13 |
| `crates/system-state` | **Keep** (locked operations); previews reuse "try on a copy" and the "what changed" comparison | C-25, R-29 |
| `crates/history` | **Keep** (locked format) | C-24 |
| `crates/assistant`: turn loop, tools, conversation, tests | **Keep and extend** (plan, notes, skills, steering, compaction, autonomy modes) | R-21 |
| `crates/assistant/src/claude.rs` | **Replace** by `agq-providers` on rig after S4.2 | C-34 |
| Compiled-in skills | **Simplify** into `SKILL.md` files, still compiled in as defaults | C-41 |
| Conversation format | **Replace** with format 2 (provider-neutral); format 1 conversations are imported read-only as transcripts | R-23 |
| `crates/studio-native` | **Keep and restructure**: tokens and components, docked panels, Settings view, Conversation polish; journeys extended | C-32 |
| `theme.rs` | **Replace** by the tokens module and generated themes | R-26 |
| `markdown.rs` | **Rewrite** on GPUI (selection across messages, tables) | C-48 |
| `gpu.rs`, `scene.wgsl` | **Replace**: GPUI draws the Surface with its own quads, paths and text in paint layers | GPUI cannot host wgpu output on Windows (C-48) |
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

### 5.6 Foundation review (2026-10-01, `main` at `f3d0dae2`)

A read-only review before Stage 10 (C-51) traced three workflows through the
code. **Observed** means read in the code at that revision; **intended** means
what this document or a README says; **not verified** means neither was
run. The four checks passed on that revision (`docs/stages.md`, Stage 10).

**What Agentique does, and what lies outside it.** The purpose is in §1.1
(C-55; it replaces C-51's organising principle in §1.5).
Inside: one System State per project (the model: intent), its
history in git, the Library, scenarios and their runs (observations),
implementation links and checks, controlled execution of builds and tests in
worktrees, and the Assistant. Outside: the project's code (in its own
repository, linked), deployments, other people, and any sandbox the host does
not have.

**Who owns what** (observed; the self-model states it per part, W10.2):

| State | Owner | Changed only by | Kept in |
|---|---|---|---|
| Element tree, locks, revision, undo steps | System State | `SystemState::apply`, `undo`, `redo`, through `Project` | `model/*.sysml`, `model/agentique.json` (undo steps in memory) |
| Model folder on disk, save journal, checkpoints | History | `Project` | `model/`, git |
| Implementation links, harness binding | Implementation | `Project::save_links` (outside undo, C-50) | `model/links.json` |
| Runs, traces, results | Simulation | runs | app data `runs\` |
| Jobs and journals, worktrees, integration | Execution | the Studio | app data `jobs\`, git worktrees |
| Conversation entries | Assistant | the Studio, after each entry | app data `conversation.json` |
| Keys, model choices | Providers, Studio | Settings | Credential Manager, `settings.json` |
| Presentation (cameras, layouts, panels) | Studio | the Studio | `studio-session.json` |

**The workflows as they run** (observed; each step is linked to its code in
the self-model, W10.2):

1. *Rename a part:* the Inspector or the Surface → `Studio::rename` →
   `Studio::apply_change` (`edit.rs`) → `Project::apply` →
   `SystemState::apply` (stale check if the change has a base, lock check,
   writable check, link, validate, revision, comparison) → `Project::save` →
   `History::save` (journal, atomic renames) → `Studio::changed` (scene input
   from the whole tree, `Scene::update`). Undo swaps a whole-tree snapshot
   and saves; reopening reads the files and the identity file, and the session
   restores the camera. Covered by `system-state` tests (`operations.rs`,
   `project.rs`), History's crash tests, the Studio's edit tests and the
   journeys `a-build`, `a-crash`, `a-reopen`.
2. *The Assistant changes the model:* the Conversation → `BackgroundTurn`
   (its own thread) → `turn::run_with` → `Model::send` → a tool call →
   `BackgroundEvent::ToolCall` → the Studio's `carry_out` → `tools::prepare`
   (names resolved, tried on a copy, base revision set) → `apply_change` (the
   same path; a lock asks) → the result back to the loop → saved entries.
   Covered by `assistant` tests (`turn.rs`, `tools.rs`), the Conversation's
   tests and the journey `a-assistant`.
3. *An implementation task:* `propose_implementation` or the Implement
   dialog → `tasks::start` (brief, job, worktree) → the worker (the same loop
   with code tools) → the Studio's `verify` → the review dialog →
   `git::integrate`. No build, trial or adoption exists. Covered by
   `assistant/tests/worker.rs`, the Studio's task tests and Execution's git
   tests.

**Corrections, by priority.**

*P1, the self-development loop depends on them (W10.1, each with a regression
test first):*

1. **Verification counts only failures.** `Verification::failures`
   (`implementation/src/task.rs`) counts a check only when it failed:
   checks not run, unsupported, inconclusive, or never configured (no links,
   no harness, no linked tests) read as success. No required set is fixed at
   approval (the worker's proposed links decide what is checked), a
   verification has no source revision so it is never outdated, and
   Integrate is enabled whenever files changed, whatever the verification
   says. A task could report "nothing it checked fails" with nothing checked.
2. **Protected paths** are compared case-sensitively, which Windows paths
   are not, and `.cargo/` and `rust-toolchain(.toml)` are not protected, so a
   worker could change the build's own configuration.
3. **Integration** recomputes the patch at integration instead of using the
   reviewed one, commits the whole index (the Operator's staged files go
   into the task's commit), does not check protected paths, and journals none
   of its steps.
4. **Stop** does not end a task's build or test processes (the task's
   executor never gets the cancel flag), and the approval channels of
   Assistant-started runs and implementation proposals stay open after Stop.
5. **Tool calls carry no binding** to a project or session; correctness
   relies on stopping the turn when another project opens.

*P2, comprehension (W10.1 text, W10.2 model):*

6. The self-model is an inventory of crates and allowed dependencies: it says
   nothing of what a part owns, its contract, its interactions or how the
   workflows run, and has no implementation links.
7. Text that contradicts the code, corrected in place: "every change goes
   through `apply_change`" (undo, redo and loading go through `Project`; the
   URL shortener sample writes its model text directly before opening it;
   links are saved outside changes); "change events are published" (they are
   returned; the Studio rebuilds its scene input from the whole tree); §4.10's
   tool table (`write_scenario`, `start_run` instead of `apply_changes`,
   `run_scenario`); "Ask before every change" is described as enforced but is
   Stage 6 work not yet built; R-41's tolerant architecture check was not
   built (the script rejects constructs it does not know).
8. Duplications that cost understanding: three pass/fail summaries of
   verdicts that disagree (`RunResult::all_passed`, `Verification::failures`,
   the Studio's "none failed"); freshness worked out in six places; the
   tool-call dispatch copied five times (the Studio, the evaluation, two
   examples, tests); rejection wording in four places; atomic file writing
   implemented about seven times. Stage 10 removes the first; the others are
   left as they are and listed here.

*P3, recorded, not changed in Stage 10 (locked core, or not on its path):*

9. Undo and redo swap snapshots without the lock check, so undoing a change
   to a part locked after it needs no confirmation (System State operations:
   the Operator's decision).
10. A change that leaves a reference `unreachable-target` is applied and
    saved; reopening binds it again by name, possibly to another element
    (language core).
11. Each edit reads the whole model folder twice and HEAD's tree once
    (W5.5's 10k edit budget).

*Not verified by the review:* the journeys and the live runs were not run
again.

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

**Spike S4.1: toolkit** (R-20). *Superseded by the Operator's decision C-48
(2026-09-28): the Studio moves to GPUI; Track B's gates were waived.* Ten
working days, hard stop, throwaway branches only; nothing merges. Exception (§7.6, 2026-09-27): Track A's code may
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

- **W5.1 Toolkit.** Migrate the Studio to GPUI and redesign its presentation
  (C-48), keeping its behaviour, journeys and budgets.
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
- **W5.13 Library** (C-49, §4.13, Scenario H): the `agq-library` crate and its
  built-in blocks; the Library panel beside the Outline with search, scopes,
  kinds and a structural preview; insertion by keyboard, palette, context
  menu and drag; "What can connect here?"; opening a definition and going
  back; Edit definition, Specialise and Override here; creating a block from
  a selection; My Library; the Assistant's Library tools and skill;
  evaluation tasks that tell appropriate reuse from blind reuse; the journey
  `h-library`.

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
- Journeys `a-build`, `a-crash`, `a-reopen`, `a-assistant`, `d-daily`,
  `e-settings` and `h-library` pass.

**Depends on:** Stage 4. **Waits:** plan, steering, notes, skill files,
compaction, autonomy modes, agents.

### 6.4 Stage 6: the agentic Assistant

**Outcome.** Scenario F works on at least two providers. (Scenario G steps
G1–G3, agents in the model, moved to Stages 7–8 with W6.9 and W6.10 under
C-50; the items keep their numbers there.)

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
- *(W6.9 and W6.10 moved to Stages 7–8, C-50.)*
- **W6.11 Evaluation set extended:** long tasks, the three modes, steering;
  run on each provider to confirm the capability table (A-8).
- **W6.12 Journey** `f-long-task`.

**Interfaces to merge first:** schemas of `update_plan`, `read_skill`,
`propose_note` and `ask_operator.preview`; the skill loader's contract; the
notes store; the autonomy mode and its effect on the executor; the event
protocol and conversation entries for plans, queued and delivered messages,
previews, note proposals and compaction summaries.

**Acceptance evidence**

- The Operator runs Scenario F with a task of at least about twenty tool calls,
  in each autonomy mode, on at least two providers: steers, stops and sends,
  refuses a lock change, undoes a turn, accepts and rejects notes, adds a skill.
- Must-hold behaviours pass in every trial of the evaluation set: never claims
  an unconfirmed change; never changes a lock without confirmation; asks on major
  decisions in the default mode; never shows SysML text.

**Depends on:** Stage 5. **Waits:** the Orchestrator.

### 6.5 Stages 7–8: the factory loop, with agents (C-50)

The Operator's direction of 2026-09-30 (C-50) builds agents in the model
(W6.9, W6.10), simulation (Stage 7) and implementation links (Stage 8) as one
bounded cross-stage phase: **one continuous loop**, delivered as working
vertical slices, while Stage 5's open items and Stage 6's Assistant items wait
their turn. It replaces the separate Stage 7 and Stage 8 plans and their spikes
S7.1 and S8.1, whose questions (Q-5, Q-2) the design of §4.14–§4.15 answers,
pending the Operator.

**Outcome.** Scenario I (§2.10), which completes Scenario A (A5–A7, C-19) and
Scenario G (G1–G9): the Operator designs a system with agents, exercises its
behaviour, has it implemented, inspects the results, sees a disagreement and
changes the system safely, in one experience at the Studio's quality bar.

**Milestones** (each buildable, visible in the Studio and covered by tests):

| Milestone | Observable outcome |
|---|---|
| M1 Executable architecture | A conventional subsystem has explicit behaviour and scenarios that genuinely run, with useful traces and failing checks |
| M2 Agentic components | An agent uses the same model and Library concepts, with deterministic stand-ins, replay, failure handling and an explicit live-evaluation path |
| M3 Real implementation | The Assistant produces or changes runnable code, links it to the model, and runs the relevant scenarios and checks against it |
| M4 Closed-loop change | Agentique detects a deliberate contradiction, explains it in context, guides a bounded repair, and keeps the intended requirements |

**Work items**

- **W6.9 Agents in the language:** `enum def` and enumeration values, the
  built-in `Agents` library, the validity rules `wrong-fallback` and
  `agent-fallback`, deviation 12, the subset manifest (R-37).
- **W6.10 Agents in the Studio and the self-model:** the agent badge and the
  Inspector's Agent section (R-38); `dependency` in the subset and the
  self-model validated by our own core (R-41); the Assistant modelled as an
  agent (C-44).
- **W7.1 Behaviour and scenarios in the language:** expressions, the behaviour
  subset, verification cases, the `Scenarios` library, System State properties,
  the manifest and deviations (§4.14).
- **W7.2 Simulation:** the crate, the run contract, model execution, traces,
  checks, results with provenance and freshness; the Library's blocks gain
  explicit behaviour where it earns its place (a retrying worker, a queue, an
  agent with a recovery path).
- **W7.3 Agents at run time:** stand-ins with injected failures, recordings
  keyed by request digest with `missing-recording`, live evaluation through an
  explicit model client with samples, intervals, failure categories and
  provenance, and keeping a good run as recordings (R-39).
- **W7.4 Studio, runs:** the Scenarios tab, structured scenario authoring, the
  run controls (Run, Pause, Step, Stop, playback), the trace timeline with
  filters, check verdicts and freshness, highlights of active elements from
  recorded events, the Behaviour and Evidence sections of the Inspector.
- **W8.1 Execution:** the typed executor, scopes, trusted-local mode, jobs with
  journals, cancellation and recovery (§4.15).
- **W8.2 Implementation:** links, code-to-model lookup, the harness protocol
  and implementation runner, the supported checks and drift (§4.15).
- **W8.3 The supervised implementation loop** and the Assistant's run and code
  tools (§4.10), with grouped approvals, patch review beside the affected parts,
  and integration that checks the base revision again.
- **W8.4 Proof:** Scenario I end to end on the URL shortener with link
  screening, the second example (a retrying notification dispatcher), the
  deliberate model and implementation breaks, and the dogfood check on this
  repository; journeys, gallery entries and budgets for the new paths.

**Acceptance evidence.** The Operator walks through Scenario I (I1–I8) in the
Studio, with the Assistant on a real provider for I4 and I6 and an explicit
live evaluation for I3, and accepts it (C-15). Automated tests, journeys and
the reference budget run support the acceptance and never replace it.

**Depends on:** Stage 5's Studio and Library (built, pending acceptance).
**Waits:** Stage 5's open items, Stage 6's Assistant items, remote workers,
unattended deployment, languages other than Rust for implementation checks,
stochastic and numerical simulation.

### 6.6 Stage 10: Agentique builds Agentique (C-51)

Brought forward by the Operator on 2026-10-01 (C-51) as the last externally
driven bootstrap. After it is accepted, Agentique is developed from within
Agentique; emergency external recovery stays possible but is not the normal
workflow. Stage 5's and Stage 6's open items and Stages 7–8's acceptance wait.

**Outcome.** Scenario C (§2.8, C1–C8).

**Work items**

- **W10.1 Foundation corrections:** the P1 items of §5.6, each with a
  regression test written first; one verdict summary shared by runs, tasks
  and implementation checks; the contradicting text of §5.6 item 7 corrected
  in place.
- **W10.2 The self-model explains the product:** the model at `model/`
  (§4.6); purposes, owned information, contracts, interactions; the three
  workflows of C2 as scenarios with their failure paths, linked to code and
  tests; "Develop Agentique"; the Inspector answering C1's questions; the
  architecture check reads `model/` and ignores constructs it does not need.
- **W10.3 The runtime boundary and the Claude Agent runtime** (§4.7, §4.10):
  `Runtime`; the companion and protocol 1; Agentique's tools through one
  in-process MCP server; the permission configuration and isolation; setup
  and health in Settings; streaming, tool activity, questions, approvals,
  cancellation, errors and usage in the Conversation; sessions kept, resumed
  and handed over explicitly.
- **W10.4 Development tasks** (§4.15): the task definition fixed at approval;
  required checks with explicit outcomes; the coding tools; the worker on
  either runtime; proposed model changes in the worktree; the task commit;
  model and code reviewed together; integration of the reviewed commit; local
  commits; pushing only as an explicit Operator action.
- **W10.5 Builds and adoption** (§4.15): builds with manifests; "Try this
  build" as a test instance; "Use this build"; the launcher, the last known
  good build, safe mode and diagnostics; app data backed up; data-format
  changes blocked.
- **W10.6 Proof:** deterministic tests of the protocol, the permission
  configuration (that other tools, settings, hooks, subprocesses and stale
  approvals cannot bypass the operation boundary), the bridge, sessions,
  required-check accounting, artifact and source correspondence, test
  instances and the launcher's fallback; journeys; the reference budget run;
  the gates below.

**Gates** (the Operator's; automated tests support them and never replace
them, C-15):

| Gate | What shows it |
|---|---|
| A. Agentique is understandable | The Operator opens the real project and navigates purpose, parts, workflows, code and checks; the Assistant explains them with model and source references and says where it is unsure. The Operator's own judgment is recorded. |
| B. The Claude Agent runtime works inside Agentique | A real, authenticated SDK conversation reads the architecture, proposes a model-aware change, handles approval and refusal, does scoped development work, runs checks and survives interruption; missing authentication, runtime failure, a broken bridge, cancellation and stale requests are tested deterministically. Without a real authenticated run the gate is unverified. |
| C. Agentique produces its first improved version (replaced by W11.7, C-53) | External implementation changes are frozen; inside that build the Assistant takes a genuine, bounded improvement found in §5.6 through C4–C7, and the Operator reviews, integrates, tries and adopts it entirely in Agentique; task, source change, check results and build are recorded together. |
| D. The new version repeats it (replaced by W11.7, C-53) | From the adopted version, a second, independent improvement in another area (one correctness, one comprehension or navigation), built, reviewed, adopted and resumed without external editing or terminal repair. |
| E. Failure stays recoverable | A failed build, an incomplete verification, an interrupted SDK session, a denied permission, a moved source base and a build that cannot start, each tried on purpose: accepted source and app data stay intact and the last known good build recovers the session. |

If external code repair is needed during C or D, it is recorded as such, the
bootstrap is fixed and the proof repeated.

**Alongside Stage 10 (C-52).** The typed-decision slice of Scenario I is built
beside this stage, in dependency order, each step reviewable on its own: CI's
checkout and lint baseline (0a); Execution's host-independent paths (0b); the
Jev adapter's correctness (1); deadlines and cancellation in Providers (2);
execution identity in the run records (3); the opt-in single-choice
evaluation of `LinkScreening` in the Studio (4); the URL shortener's real
client (5); and the rig migration (U). It serves W7.3, W7.4 and W8.4 (I3, I4,
I8). It is external implementation work: it changes none of the gates above
and is not part of their proof. Paid evaluations wait for the Operator's
consent; progress and the coverage checklist are in `docs/stages.md`.

**Depends on:** Stages 7–8 (built, pending acceptance).
**Waits:** Stage 5's and Stage 6's open items; Scenario B; a general
multi-agent framework; cloud or remote workers; more implementation languages
or simulators; marketplaces; deployment platforms; another state store. (The
Orchestrator for Agentique's own objectives, and pushing within an objective's
permissions, were brought forward by C-53: Stage 11, §6.8.)

### 6.7 Later stages (order to be confirmed when reached)

Stage 11 (§6.8) was brought forward by C-53 and comes before these.

- **Stage 9. Scenario B (larger system):** readable overviews at scale,
  grouping, branches in the UI, merging by identity, keeping concepts general.
- **Reading an existing codebase into a model** (Q-7): taken out of Stage 10,
  which uses Agentique's hand-written self-model (C-51).
- **Afterwards:** objectives on projects other than Agentique's own and
  further assistant roles (C-5, Q-4); two-way reconciliation (C-7); links to
  deployed systems; local models, Gemini and typed fast-decision APIs other
  than Jev (Q-12); spending limits on the Operator's own conversation if
  wanted.

### 6.8 Stage 11: Agentique improves itself (C-53)

Brought forward by the Operator on 2026-10-03 (C-53). It builds on Stage 10's
runtime, tasks, builds and launcher and replaces its supervised proof (gates C
and D) with an autonomous one: the Operator gives objectives and watches.
Stage 10's other gates, Stages 7–8's acceptance and the work of C-52 stand.

**Outcome.** Scenario J (§2.11, J1–J8).

**Work items** (in dependency order, each integrated as a working slice):

- **W11.1 Direction and self-model:** C-53 in this document, `AGENTS.md`, the
  self-model's parts and contracts (`ClaudeAgentRuntime`, `Studio`,
  `Launcher`, the new `Orchestrator`), `CLAUDE.md` importing `AGENTS.md` for
  the SDK.
- **W11.2 The development runtime** (§4.7): the SDK's tools under the
  permission policy, the project's instructions, skills, hooks, subagents and
  MCP servers, sessions, compaction, background commands, steering, permission
  requests to the Studio, Anthropic-compatible endpoints, costs at the model's
  own prices, configuration in Settings; a capability checklist against the
  Claude Code CLI in `docs/stages.md`.
- **W11.3 The control interface** (§4.16): observation, actions through the
  real handlers and input, stable ids, stale refusal, the event trace, the
  local endpoint, Agentique's tools for it, visible agents, Pause, Step,
  Resume; journeys that drive the visible application through it.
- **W11.4 Lifecycle** (§4.15, §4.16): the supervising launcher, exact-commit
  builds, test instances exercised through the control interface, automatic
  adoption, continuation, quiescing, fallback and recovery.
- **W11.5 The Orchestrator** (§4.16): objectives and cycles end to end,
  budgets, bounded attempts, no-progress detection, independent review, the
  baseline guard, branches, pull requests, merging; the Objectives panel.
- **W11.6 Typed decisions in operation** (§4.16): at least one Jev-assisted
  application-control workflow, compared with deterministic and LLM-only
  baselines on task success, decision errors, latency and cost.
- **W11.7 Proof:** two successive genuine improvements from inside Agentique
  (one correctness, one usability or comprehension, the second started from
  the version the first adopted), each operated, implemented, reviewed,
  verified, merged, built, tried, adopted and resumed with no human command
  line, external editing or hidden repair after bootstrap; interruption and
  resume, a failing check, a stale action and a failed launch with recovery
  shown on purpose; the repository's checks, regression tests, Windows
  journeys and the performance and memory checks of the changed paths; live
  providers within the configured budgets.

**Gates** (the Operator's, C-15): the Operator enters an objective, watches
Agentique improve itself through J1–J8 twice, and accepts the result. External
repair needed during the proof is recorded as such, the cause fixed and the
proof repeated.

**Depends on:** Stage 10 (built, pending acceptance) and C-52's typed
decisions in Providers.
**Waits:** objectives on other projects; remote workers; a sandbox the host
does not offer; spending limits on the Operator's own conversation.

### 6.9 Stage 12: Agentique tests and improves itself (C-54)

Brought forward by the Operator on 2026-10-04 (C-54). It builds on Stage 11's
Orchestrator, control interface, typed decisions and launcher; Stage 11's
gate (the Operator's own run and acceptance) stands and is met by the same
run if the Operator accepts it.

**Outcome.** Scenario K (§2.12, K1–K8).

**Work items** (in dependency order, each integrated as a working slice):

- **W12.1 Direction and self-model:** C-54 in this document, `AGENTS.md`, the
  self-model's contracts (`Orchestrator`, `Studio`, `ClaudeAgentRuntime`,
  `Assistant`, `Providers`) and its new requirements.
- **W12.2 The control interface, complete and observable** (§4.16): every
  interactive control observed with a readable label (palette rows and its
  search field, switches, the Inspector's and Settings' fields, the title
  bar's controls), schemas checked as declared, title-bar buttons that answer
  operating-system clicks, control ownership, and observer mode.
- **W12.3 Models per role and credentials** (§4.16): the routing in Settings,
  effective models with fallbacks and reasons, effort, spend per role, the
  credential check and the detection of a local Claude login.
- **W12.4 Exploration and testing knowledge** (§4.16): the explorer's run,
  invariants, findings, reproduction and reduction, the testing record, and
  the ways of deciding (rules, Jev, the explorer's model, Jev escalating)
  compared on fixed, held-out exploration tasks for useful coverage,
  progress, unwanted actions, latency and cost.
- **W12.5 Exploration in cycles, stronger gates and a continuing loop**
  (§4.16): Explore and Reproduce before Propose, the replay as a criterion
  that must fail on the original build, evidence on the base, evaluation of
  user-facing changes, failure identity, configurable budgets, test
  instances in a stated condition and with their credentials, cycles that
  continue by themselves through adoption and recovery, and the bounds on
  worktrees, builds and branches.
- **W12.6 The Conversation as the one place** (§4.16): objectives started
  and followed in the Conversation on the same records and commands as the
  dashboard, threads (messages, directives, results, system events, folded
  activity with diffs), directives as records, the lead's `delegate` tool
  and child objectives within the parent's permissions and budgets, with
  their results returned to the parent.
- **W12.7 Proof:** from one intent, with no external Claude Code after the
  bootstrap: explorers take materially different paths from real
  observations; one intent entered in the Conversation; agents' directives
  and delegation streamed into its thread; agents operating the
  Conversation of a test instance through its real input; exploration
  reaches behaviour not covered before and recovers from an unexpected state;
  at least one discovered problem goes through implementation, independent
  checks, review, pull request, merge, build and adoption; the next
  exploration uses what the previous one learned; interruption and resume, a
  failed check repaired, a stale action recovered and a launcher fallback,
  shown on purpose. Naturally found problems are kept apart from injected
  faults (a fault a script outside Agentique causes, as in W11.7, recorded as
  such). The repository's checks, the Windows journeys and the reference run
  on the final `main`. Where a role's default model has no credential it may
  use (on the reference machine, roles that call Claude directly, since only
  the Operator's subscription token is there), the proof shows that role on
  its fallback, with the reason, and says the default is not tried. Under
  C-55 this proof runs as W13.7, with a modelling or simulation objective.

**Gates** (the Operator's, C-15): the Operator gives an intent, watches
Agentique explore, delegate, fix and adopt through K1–K8, and accepts the
result.

**Depends on:** Stage 11 (built, pending acceptance).
**Waits:** exploration of projects other than Agentique; a claude.ai login
offered by Agentique (needs Anthropic's approval, §4.16); remote test
machines.

### 6.10 Stage 13: Agentique models, simulates and evolves systems, itself included (C-55)

Directed by the Operator on 2026-10-09 (C-55). It builds on Stage 12's
mechanisms (merged; their proof outstanding) and folds W12.7 into its own
proof. Stage 12's gate stands and is met by the same run if the Operator
accepts it.

**Outcome.** The engineering questions below are answered by Agentique, on
its own model and on a materially different system, with evidence that says
what it is; and the autonomous loop improves this modelling capability while
staying aligned with the purpose (§1.1).

- *Composition and sharing:* what is owned, composed, shared or referred to;
  can one element be used in several roles without being copied or counted
  twice; can a system refer to another instance of its own kind?
- *Requirements and evidence:* which requirement does a configuration
  satisfy, under which assumptions, by calculation on the model, by scenario,
  by implementation check, or only by declaration?
- *Behaviour:* does authorisation precede energising; does a timeout recover;
  does the autonomous cycle merge only when its gates pass and recover when a
  build fails?
- *Evolution:* which requirement does a change serve, what did it actually
  change, and what has changed since the Operator's approved baseline?

**Work items** (in dependency order, each integrated as a working slice):

- **W13.1 Direction:** C-55 and the purpose in this document (§1.1),
  `AGENTS.md` referring to it, the locked parts named in §7.6, the Stage 12
  record reconciled, `docs/stages.md` Stage 13, and a way for external
  agents to change a model through Agentique's own `apply_changes` without a
  window.
- **W13.2 Referential usages** (§4.3): `ref part` and `ref item` across the
  whole path; composite and referential kept apart in validation
  (composition cycles, a composite part bound to another part), in model
  execution (a bound reference is the same instance) and on the Surface.
- **W13.3 Requirement constraints and evidence** (§4.3, §4.14): assumed and
  required constraints and subrequirements across the whole path; their
  evaluation on the modelled configuration of what satisfies them; the
  claims of §4.14 kept apart in the Requirements panel, the Inspector and
  the agents' tools (a declaration is never shown as satisfaction).
- **W13.4 The self-model** (§4.6): the purpose's obligations as
  subrequirements of the locked `purpose` (its lock confirmed for that change
  on the Operator's C-55 instruction); the autonomous lifecycle (objective, exploration, delegation, proposal,
  implementation, checks, review, merge, build, adoption, recovery,
  continuation) as the Orchestrator's behaviour with its role agents and
  scenarios for the success and failure paths, linked to the tests that
  check the implementation; the test instance as a reference to another
  Agentique; logical responsibilities kept apart from their crates.
- **W13.5 Alignment in the loop** (§4.16): proposals that name what they
  serve, affect, benefit and cost; traceability by identity; the cumulative
  review against the approved baseline; the purpose and the governing text
  protected from cycles; dispositions of findings before they are fixed.
- **W13.6 A second system:** an inspection drone and its charging station,
  after the Operator's supplied guide, modelled for its architecture,
  contracts, discrete behaviour and requirement calculations, not its physics
  (§1.6): its boundary, assumptions, reusable
  definitions, configurations, interfaces, behaviour and requirements; a
  bounded question answered by simulation and calculation, with a successful
  case and failure cases (authorisation refused, a timeout and its recovery,
  a limit exceeded, a shared element counted once); one reusable definition
  in several contextual usages without copying it.
- **W13.8 The start of an objective** (the Operator's amendment of C-54,
  made during W13.7): the intent alone, read by a typed decision for what
  the objective does and shown before Start; spend and time unlimited
  unless set.
- **W13.7 Proof:** W12.7's proof with a modelling or simulation objective,
  and a second cycle in the adopted build that uses the first one's
  knowledge; the alignment and evidence checks visible in both;
  interruption, cancellation, a failed validation and recovery shown on
  purpose; external interventions recorded as such. The repository's checks,
  the journeys and the reference run on the final `main`.

**Gates** (the Operator's, C-15): the Operator inspects the self-model and
the second system in the Studio, runs their scenarios and calculations,
watches the autonomous cycles, and accepts the result.

**Depends on:** Stage 12 (merged, proof outstanding).
**Waits:** the ISQ and SI quantity libraries (not among the pinned
standards; quantities stay numbers in documented units, which W13.6 records
by extending deviation 14 from durations to every quantity);
individuals, time slices and snapshots; `flow`, `allocation`, metadata and
views; several state machines per part; `state def` and `action def` reused
through parameters (until a scenario needs their binding of ports).

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
| C-5 | One Assistant with tools and skills first; it becomes an Orchestrator of assistants later. Brought forward by C-53 for Agentique's own objectives |
| C-6 | The Assistant acts where it is confident and asks the Operator about major decisions. Refined by C-38: this is the default autonomy mode; in "Ask only on locks" the Operator waives these questions for a conversation |
| C-7 | Model to code: "the model is the contract; code linked and checked" is the working direction, two-way reconciliation is the long-term goal. Both are provisional pending experiments. Linking to deployed systems comes later |
| C-8 | All Assistant actions are visible in real time |
| C-9 | Single user (the Operator). A native Rust application on Windows. Several model providers through rig (C-34, C-35), replacing "the Claude API first". The Operator delegated the history/version-control decision (now C-24) |
| C-10 | The definition of slop and non-slop in §1.3, including "stable core, experimental edges" |
| C-11 | Parts can be locked. Changing a locked part requires the Operator's confirmation. Protecting established parts against idea-driven drift is essential. Amended by C-53: inside an objective, the objective naming a locked element is that confirmation; the locked core (R-16) is never named |
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
| C-35 | Providers in this phase: Anthropic, OpenAI, OpenRouter and DeepSeek (model `deepseek-flash`, effort `high`), and TypeSafe AI's Jev as a model provider for fast ("system 1") agents in designed systems, never for the Assistant. Amended 2026-09-27: the product works end to end with only a DeepSeek key configured. Amended by C-53: Jev also makes bounded typed decisions in Agentique's own operation (the Orchestrator), never as the reasoning agent |
| C-36 | API keys are stored in the Windows Credential Manager |
| C-37 | Costs are shown; there are no spending limits. Amended by C-53: an objective's autonomous work has budgets (spend, attempts, time); by the Operator's amendment of C-54 (§7.6), spend and time only when set |
| C-38 | Three autonomy modes, after Claude Code and Codex: ask before every change; act, but ask on major decisions; act freely, asking only on locks |
| C-39 | Long tasks show a visible plan, and the Operator can steer (queue messages, stop and send, stop, undo the turn) |
| C-40 | Memory: short notes the Assistant proposes and the Operator approves, per project and app-wide; nothing is remembered silently |
| C-41 | Skills: built-in skills plus the Operator's own skill files, app-wide or per project |
| C-42 | The concept of AI-driven parts is named **agent**. An agent is modelled as a part definition that specialises a built-in library definition (`Agents::Agent`) |
| C-43 | Agents in simulation run as deterministic stand-ins or recorded answers; live calls are separate evaluation runs |
| C-44 | Agentique's own Assistant is modelled in its self-model (`model/`) as the first agent |
| C-45 | Agents first appear in the Studio with the agentic Assistant (Stage 6) |
| C-46 | An agent in a designed system is implemented in the project's own code, linked and checked (provisional until Q-2) |
| C-47 | `ROADMAP.md` replaces `REALIGNMENT.md` as the single governing text; `REALIGNMENT.md` is retired to git history |
| C-48 | The Studio moves from egui to GPUI (a pinned snapshot of Zed's GPUI, `gpui-pre`, with the unstyled `gpui-base` primitives), with its presentation redesigned; the Operator accepts the governance risk and waives S4.1 Track B's gates. The Surface is drawn with GPUI's own primitives, since GPUI cannot show wgpu output on Windows |
| C-49 | Agentique has a **Library** of reusable building blocks: reusable KerML/SysML definitions from a small built-in library, the project and My Library, found, previewed, inserted, connected, specialised, overridden and created from existing architecture by the Operator and the Assistant alike. Using a block creates a usage typed by the definition, and a project keeps its own copies of what it uses; there is no parallel component model, marketplace or remote registry (the Operator's decision, 2026-09-29) |
| C-50 | **The factory loop.** Agents in the model, simulation and implementation are built as one bounded cross-stage phase and one continuous loop (intent → architecture → scenarios → execution → implementation → checks → informed change), delivered as working vertical slices with the URL shortener's link screening as the proof (Scenario I). The model (intent), the implementation (actual code) and run results (observations) stay separate; scenarios are the shared anchor; runs are labelled by what actually ran; a runner that cannot evaluate something says so; code execution goes through a controlled executor, never the model change boundary. The minimum additive language and persistence extensions its proving journeys need are authorised, each named and justified in §7.6 before it is built (the Operator's direction, 2026-09-30) |
| C-51 | **Agentique builds Agentique.** Stage 10 (Scenario C) is brought forward as the last externally driven bootstrap, with three inseparable outcomes: Agentique becomes understandable again (its self-model explains a working product), its Assistant gains a runtime on the official Claude Agent SDK, and it can produce, validate and adopt its next version and recover from a bad one, proven over two generations by the Operator (amended by C-53: the two generations are proven autonomously, Stage 11). Authorises the bounded milestone and the SDK runtime it needs, including an SDK-owned agent loop for that runtime only, with the exception to C-34 and R-21 recorded; it does not authorise an unrelated rewrite, weakening protected guarantees, or declaring the Operator's acceptance (the Operator's direction, 2026-10-01) |
| C-52 | **Typed decisions for Scenario I, alongside Stage 10.** The System One investigation's plan (updated 2026-10-03 after PR #86) is brought forward beside Stage 10 as explicitly reprioritised work: restoring CI, correcting the Execution safeguard's paths, making the Jev adapter correct (complete bounded replies, request-bound validation, explicit usage), end-to-end deadlines and cancellation, exact execution identity in the existing run records, the opt-in single-choice `LinkScreening` evaluation in the Studio, a real Rust `LinkScreening` client in the URL shortener's code with frozen-response conformance tests, and a separate, deliberate rig migration (C-34). Authorises those designs and their necessary supporting changes, including the additive persistence change recorded in §7.6. It does not authorise paid inference, live evaluations, production activation, Jev for the Assistant (C-35 stands), a router, registry, new runtime or library extraction; it changes no Stage 10 gate, does not count as its two-generation proof and declares no acceptance (the Operator's direction, 2026-10-03) |
| C-53 | **Agentique improves itself.** Autonomous, self-improving development through Agentique: AI agents are its primary users; the Operator supplies intent as objectives, observes activity and outcomes visually, and steers or stops the work. Agentique builds, runs, tests and evaluates Agentique and adopts a new version when the objective's acceptance criteria pass, with no routine approvals and no terminal repair. Supersedes the rules that required supervised-only development, the Operator's approval of every development cycle and integration, "Agentique never pushes", the postponement of the Orchestrator, the Claude Agent runtime's restriction to Agentique's own tools, Jev never serving Agentique's own operation (C-35) and the absence of budgets for autonomous work (C-37). Authorises the necessary changes to locked parts (the Claude Agent runtime, the launcher, Execution's use, the companion protocol) when they are documented, reviewed, tested and recoverable, each named in §7.6. Keeps: model changes through the one operation boundary with validation and identity (C-2); locks (an objective may name locked elements it may change); deterministic checks and independent review decide, and an agent's or a typed decision's judgment never overrides a failing check; weakening a check never counts as improvement; Stop and recovery always work; the host's permissions, credentials, repository rules and configured budgets are respected; no force-push, no bypassed branch protection; unrelated guarantees stand (the Operator's direction, 2026-10-03) |
| C-54 | **Agentique tests and improves itself.** From one intent, Agentique explores its own running application through the real GUI, reproduces what it finds, carries a reproduced problem through a cycle (C-53) to adoption, and explores the adopted version again with what it learned; testing knowledge, scenarios, skills and the ways of deciding improve only on measured evidence. Each agent role runs on its own configured model: Claude Opus 5.5 for the lead, the independent reviewer and escalation (difficult reasoning); Claude Sonnet 5.5 for the implementer, the evaluator and the Assistant; DeepSeek's `deepseek-flash` for the explorer; Jev for bounded typed decisions; every fallback explicit and shown with its reason. Agents may delegate child objectives within the parent's permissions and budgets, as directives the Orchestrator validates and records; the Conversation is the one place where the Operator gives intent and follows and steers every agent, the Objectives panel a dashboard of the same work (clarified by the Operator the same day). Authorises the necessary changes to the Orchestrator, the Claude Agent runtime, the control interface, the Studio's rules for agents and Settings, each named in §7.6. Keeps everything C-53 keeps, and adds: Agentique never offers a claude.ai login (that needs Anthropic's approval), never reads the CLI's stored login and never extracts or copies a credential, while the Operator may give it their own subscription token from `claude setup-token` for their own sessions in the Claude Agent runtime (decided by the Operator later the same day); a finding is a deterministic check that failed and reproduced, never a model's opinion alone; a fix counts only when its regression fails on the original implementation; user-facing changes are verified in the GUI; the tasks that judge testing knowledge and the ways of deciding are fixed and held out; external Claude Code bootstraps the stage and does not steer its proof (the Operator's direction, 2026-10-04) |
| C-55 | **Agentique models, simulates and evolves systems, itself included.** Agentique's enduring purpose is recorded once, in §1.1: a system for understanding, modelling, simulating, verifying, implementing and evolving systems through explicit architecture grounded in KerML and SysML v2, which is itself such a system and uses for itself the concepts, operations, contracts and verification it offers; agents do the work, people give intent, inspect, observe and keep control of the purpose and consequential decisions; a system of systems that builds and evolves systems, itself included. The root system and "generalise the mechanism, specialise the application" are architectural principles (§1.1). Stage 13 (§6.10) strengthens the KerML/SysML foundations for the scenarios that need them (referential usages, requirement constraints and their evaluation, reusable calculations), each complete on the whole path; improves the self-model substantively, with the purpose's obligations and the autonomous lifecycle modelled and checked; proves the same abstractions on a second, materially different system; makes every autonomous proposal name the requirement it serves, the elements it affects, its benefit, its evidence (its frozen criteria) and its effect on root complexity, reviewed cumulatively against the Operator's approved baseline; keeps verification claims apart (§4.14); and completes Stage 12's proof with a modelling objective. Authorises the narrowly scoped changes to the language core, the System State operations, Simulation, the Orchestrator and the related Studio and Assistant code these scenarios need, each named in §7.6 before it is built, with compatibility kept and migrations explicit. Keeps everything C-53 and C-54 keep, and adds: the purpose, the governing text and their protections change only by the Operator's decision (agents may propose, never make, such a change); proposals never weaken the purpose, acceptance criteria or review requirements to succeed; mechanical checks and reasoned review stay distinct, and no score or agent approval proves alignment; standard syntax never receives a convenient meaning of Agentique's own, and unsupported semantics are reported; the pinned standards are preserved; external Claude Code bootstraps and repairs, records its interventions, and does not supply intermediate objectives or edit the solution during a claimed autonomous proof (the Operator's direction, 2026-10-09) |

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
| R-28 | Incremental Surface updates: a change lays out the cards again in full with `LayoutMemory` (10–20 ms at 10k) and re-routes only the edges it touches; label layouts cached *(S5.1; decided overnight 2026-09-28, pending Operator confirmation)* | §5.2: 2.7 s per edit at 10k |
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
| R-41 | Add the standard `dependency` relationship to the subset so that CI can validate the self-model with `agq-language`; `check_architecture.py` also forbids the rig crates, tokio, reqwest and the credential-store crates outside Providers (`reqwest` in `agq-assistant` is a temporary exception until W5.7) and tolerates model constructs it does not need | C-20; R-15 |
| R-42 | Cost display per turn and per day, from provider usage and a dated local price table marked as an estimate, per provider in Settings | C-37; the Models API has no prices [16] |
| R-43 | One per-project folder in the app's local data (`projects\<folder>-<hash>\`: conversation, notes, skills); existing conversations move there; the session file keeps only presentation state; preferences move to `settings.json` *(provisional for notes and skills, Q-10)* | C-26; one place per kind of data |
| R-44 | Investigate the memory footprint in Stage 5 (the renderer on Windows, font atlas, buffers) before fixing the memory budgets | §5.2: 322 MB at rest, not understood |
| R-45 | The evaluation set also compares default models and effort (for example `claude-opus-5` at `high` against `claude-opus-5-5` at `medium` and `high`) and every Assistant provider (Anthropic, OpenAI, OpenRouter, DeepSeek) before any default changes | Q-19; C-27 |
| R-46 | A three-step first run (what Agentique is; connect a provider or skip; create or open a project), with the URL shortener as a sample | Scenario E1 |
| R-47 | The Library as in §4.13: a new part Library (crate `agq-library`) above the language core and the System State; blocks copied with their dependency closure into the project's `Library` package, identical copies reused and conflicts never overwritten; origin derived from qualified names and content, never stored; fit with a port decided by the language's own rule through the read-only `Semantics` query (the one locked-core addition); My Library as one SysML file in the app's local data; four Assistant tools, saving to My Library only at the Operator's request and after a confirmation; the journey `h-library` with a scripted stand-in | C-49; §8.1 rules 4–6; D-1 |
| R-48 | Scenarios are standard verification cases (subject, objective with `verify`, steps, `assert constraint` checks); stand-ins are usages of the built-in `Scenarios::StandIn`; the same scenario runs in the modes `model`, `replay`, `implementation`, `live` and `walkthrough` of one run contract *(decided in this session under C-50, pending the Operator)* | C-50; SysML 7.22; §4.14 |
| R-49 | Model execution semantics as in §4.14: fixed snapshot, logical milliseconds, ordering by time then scheduling order, run to completion, `ambiguous-transition` and `unhandled-message` as stop reasons, explicit limits and stop reasons, no randomness yet *(decided in this session under C-50, pending the Operator)* | C-50; §5.5; Q-5 |
| R-50 | Results carry provenance digests; freshness is computed, never stored; checks are passed, failed, not run, unsupported, blocked or inconclusive; the claims of §4.14 stay apart (five under C-50, nine as amended by C-55) | C-50, C-55; §8.3 |
| R-51 | Implementation as in §4.15: links in `model/links.json`; one path (Rust with Cargo); four checks with stated coverage; a harness speaking one JSON line per step; the Execution service with trusted-local mode, scrubbed environments, scoped writes, jobs with journals; one supervised worker with bounded repair and protected checks *(decided in this session under C-50, pending the Operator)* | C-50; Q-2; Q-15 |

### 7.3 Assumptions

| ID | Assumption | Status or how it gets tested |
|---|---|---|
| A-1 | The Generation 2 core, without closure and audits, validates fast enough to feel live | Disproved in Stage 1 (72 s per edit); the core was rebuilt (C-23) |
| A-2 | A small KerML/SysML subset is enough for Scenario A | Holds for the architecture steps; simulation extends it (Stage 7) |
| A-3 | SysML text in git with an identity file preserves element identity well enough | Holds in the Stage 1–2 tests; unnamed elements are matched by position (a known limit) |
| A-4 | The Claude API with tool use produces coherent architecture changes through typed tools | Open: tested live in Stage 4 (§2.2, R-19) |
| A-5 | egui (with custom rendering) can reach the quality bar, including the Conversation | Closed: the Operator chose GPUI (C-48) |
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
| Q-2 | Which model-to-code checks are feasible and worthwhile, for parts and for agents | Decided in this session under C-50, pending the Operator: the four checks of §4.15, each with its stated coverage (R-51) |
| Q-3 | What exactly counts as a "major decision" | Partly resolved: the autonomy modes (C-38) settle the levels; the definition lives in the `decisions` skill and is refined with the evaluation set |
| Q-4 | Orchestrator design: which assistant roles, how they coordinate (multi-agent systems cost about 15 times the tokens of a chat [22]) | Partly resolved by C-53 for Agentique's own objectives (§4.16: lead, implementer, reviewer, evaluator, coordinated by deterministic phases); other projects stay open |
| Q-5 | What simulation semantics are needed to test contracts (message flows, states, actions, time?) | Decided in this session under C-50, pending the Operator: §4.14 (R-49) |
| Q-6 | Whether egui is the right toolkit for the quality bar | Decided by the Operator: GPUI (C-48) |
| Q-7 | How to read an existing codebase into a model | No (Stage 10) |
| Q-8 | Which parts of the SysML standard the Operator would miss under the subset | No (reviewed as it grows) |
| Q-9 | Where conversation history is stored | Resolved (C-26) |
| Q-10 | Should per-project skills and notes live in the project folder, versioned with the code, instead of the app's local data? | Decided overnight 2026-09-27, pending Operator confirmation: the app's local data, as in R-43 |
| Q-11 | Should Jev or another typed fast-decision API become an agent's model provider, once rig releases `rig-typesafeai` [9]? | Resolved by the Operator (C-35): Jev is a model provider for fast agents now, through a thin client until rig releases it (C-34) |
| Q-12 | When do local models and Gemini arrive? | No; after Stage 8 unless the Operator pulls them earlier |
| Q-13 | How is escalation from a fast agent to a deliberate one modelled: two agents and a routing part, or a state machine? | Open; C-50 keeps it out of the fallback: a fallback is never an agent, and escalation is modelled explicitly when a scenario needs it |
| Q-14 | How does a requirement state an agent's required pass rate (needs constraint expressions or a convention on attributes)? | Open; a live evaluation reports its numbers with an interval and the Operator judges them (C-50) |
| Q-15 | Which code tools does the Assistant get for implementation, and how do the autonomy modes extend to side effects outside the System State? | Decided in this session under C-50, pending the Operator: §4.2, §4.10, §4.15 (R-51) |
| Q-16 | Where are recordings of agent answers stored (project folder or app data), and are they committed? | Decided in this session under C-50, pending the Operator: kept recordings are fixtures in `<project>/recordings/`, committed only if the Operator chooses (§4.9) |
| Q-17 | Inter or Segoe UI Variable for the interface font, judged side by side at 100% and 125% scaling | Decided overnight 2026-09-27, pending Operator confirmation: keep Inter; the side-by-side judgment stays with the Operator (W5.2) |
| Q-18 | Server-side safety fallbacks under rig: an upstream contribution, a thin adapter, or none with "Retry with another model" | Decided overnight 2026-09-27, pending Operator confirmation: keep C-27 with a thin adapter in `agq-providers` for the `fallback` content block; the upstream contribution is written as a proposal, not filed |
| Q-19 | Should the default model move from `claude-opus-5` at `high` to `claude-opus-5-5` (cheaper; its default effort is `medium` per Anthropic's model documentation, not re-checked against the live page [10][12])? Newer models also tie thinking blocks to their model and conversation (S4.2, P2) | No; the evaluation set informs the Operator (R-45) |
| Q-20 | Do live evaluations of agents belong to the Simulation part (with a dependency on Providers) or to a separate part? | Decided in this session under C-50, pending the Operator: Simulation runs them through a model client the Studio gives it, so it never depends on Providers (§4.6) |

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
| 2026-09-27 | S4.1 Track A built on a branch kept for W5.1 review: egui and eframe 0.36.2 with wgpu 30.0.1; the Rust toolchain moves from 1.92.0 to 1.97.1 (egui 0.36 needs 1.95; 1.97.1 was already installed); Inter's variable font replaces the three static weights; the Studio uses eframe's low-latency surface (one frame in flight), eframe's own default since 0.35. S4.1's provisional decision is to stay with egui 0.36 and keep Track A as W5.1: Track A failed no gate it was measured on (G1 partly, G3, G4, G5, G8 partly); G2, G6 and G7 and all of Track B were not tried, so the rule does not settle it yet (`docs/stages.md`). Decided overnight 2026-09-28, pending Operator confirmation | §7.6 exception for Track A (overnight instructions, point 3); egui's minimum Rust version |
| 2026-09-28 | S5.1 decided by its rule: routing was 98% of a rebuild, so an edit lays out the cards in full with `LayoutMemory` and re-routes only edges whose ends or lane changed or whose route crosses the old or new place of a changed card (R-28 amended). The Surface half of an edit on the 10k fixture takes 71–86 ms (77.7 ms on CI), 5–7 ms at 1k; unrelated cards do not move. A kept route may keep a detour a full build would not choose, and routes then depend on the edit history (undo does not always restore earlier routes) until the next full build. With the Studio on `Scene::update`, an edit on a 10,204-element project reaches a built frame in 127 ms (median, CPU side, before presenting): the scene takes 17 ms, applying and saving the change and rebuilding the scene input about 95 ms. That is over C-33's 100 ms, so A-9's first half is not yet met; W5.5 continues with apply and save. The spike's code is kept and merged as the start of W5.5, a deviation from "throwaway" (§8.8). Decided overnight 2026-09-28, pending Operator confirmation | Criterion of S5.1; A-9 |
| 2026-09-28 | Stage 5 details decided overnight: appearance (theme, contrast, reduced motion) lives in `settings.json`, and a Stage 4 session hands its values over once; "per day" costs (R-42) are UTC days, since the workspace has no date library and the local time zone needs a Windows call; the first run's welcome (R-46) shows until a first project is opened, and the URL shortener sample is created as a new project beside the default folder; Shift+1 and Shift+2 are read by the key's place on the keyboard. Decided overnight 2026-09-28, pending Operator confirmation | §3.7, R-42, R-46, Scenario D2 |
| 2026-09-28 | **C-48, the Operator's decision:** the Studio moves from egui to GPUI and its presentation is redesigned (W5.1). The Operator accepts the governance risk (the official `gpui` crate has been frozen at 0.2.2 since 2025-10-22; the Studio pins the weekly snapshot `gpui-pre =0.3.7` with the unstyled `gpui-base =0.7.0`) and waives S4.1 Track B's gates; A-5 and Q-6 are closed. The Operator also accepts a screen-reader regression if GPUI falls short; `gpui-pre-windows` 0.3.7 carries AccessKit, so the §3.5 names are kept where it exposes them. The wgpu renderer (`gpu.rs`, `scene.wgsl`) is replaced by GPUI's quads, paths and text; GPU timestamps are no longer measured (§3.3). A probe on the reference machine drew the 10k stress fixture with every edge and label at frame interval p95 about 14 ms, and 1k at the display rate | The Operator's instruction in this session; GPUI's ceiling on text, motion and components (S4.1 "Why these two") |
| 2026-09-28 | W5.1 on GPUI, results: egui, eframe and wgpu are gone from the Studio; the Surface, Panels, Conversation, palette, dialogs, welcome and Settings are redrawn on one design system (`ui/`: theme roles from the tokens, Lucide icons, buttons, fields, menus, dialogs, tooltips, chips, switches, a segmented control) with springs that reduced motion turns off, and `--fixture components` shows every token and component, the real Surface and the real Conversation. All six journeys pass through GPUI's own input dispatch (`a-build` 74 steps, `a-crash` exits 3 by design, `a-reopen` 15, `a-assistant` 19, `d-daily` 20, `e-settings` 16), in the light and dark themes. Reference run (release, 165 Hz display): start to first paint 251 ms warm (budget 400); 1k pan and zoom frame interval p95 6.3 and 6.2 ms (8.3); 10k 13.1 and 12.6–12.7 ms over two runs (16.7); chat, 200 messages, scroll p95 6.3 ms and streaming 6.6 ms. Decided on the way: Inter ships as three static instances (400, 500, 600) made from Inter Variable, because GPUI's Windows text system does not select a variable font's weights (with the variable file every weight drew as one face); in a view with more than 6,000 edges, route jogs under a pixel are not drawn, and a Surface query that covers a quarter of the model or more walks the scene instead of the grid (same content and order); below Features, lines too small to read are drawn as bars ("greeked") and a port label that would run into its card's text is left out (the port and its hover details stay). Screen-reader names are kept through GPUI's AccessKit roles; the three egui UI tests became tests of the functions the views draw with. Decided in this session under the Operator's instruction, pending the Operator's use and acceptance (§8.3) | C-48; W5.1; §3.3 budgets; §3.2 |
| 2026-09-29 | **C-49, the Operator's decision:** a Library of reusable building blocks (§4.13, Scenario H), built in Stage 5 as W5.13 | The Operator's instruction in this session: build with reusable KerML/SysML definitions instead of assembling every system from individual elements |
| 2026-09-29 | Locked core, under C-49: `agq-language` exposes its existing lookup (features, types, generals, specialisation) and its port rule (`incompatible-ends`, deviation 8) as the read-only query `Semantics`. No construct, rule or meaning changes; the System State operations and the persistence format are unchanged | The Library must use the model's own rules, not a copy of them (C-49) |
| 2026-09-29 | R-47 adopted with C-49: the part Library and its crate in the self-model with its dependencies; My Library's location (§4.9); four Assistant tools (§4.10); a Library search budget (§3.3) with a CI ceiling; glossary entries for Library, building block and My Library | §8.1 rules 5 and 6; §8.4 |
| 2026-09-29 | W5.13 in the Studio, results: the Library tab beside the Outline (search, scopes, kinds, structural preview), insertion by keyboard, palette, context menu and drag, "What can connect here?", opening definitions with a breadcrumb and Back, the Inspector's Definition section with Override and Reset, Specialise, Create building block from selection, Save to My Library, block links in the Conversation, and the Library in the component gallery. Journeys pass on debug builds: `h-library` (75 steps) at 100%, 150% and 200% UI scale with reduced motion, and the six earlier journeys (`a-crash` exits 3 by design); 420 tests pass. Library search 2.4 ms per keystroke at 5,000 blocks (release; budget 8 ms). Not done: the reference budget run of §8.6 (the release build was stopped when the machine ran low on memory), the h8 evaluation tasks live, screen readers. Pending the Operator's use and acceptance (§8.3) | C-49; W5.13; §8.6 |
| 2026-09-29 | Decided in this session, pending the Operator: when the docked columns would leave the Surface less than a quarter of the window (150% and 200% UI scale), they narrow for display, first giving up their width above the 200-point minimum in proportion, then sharing equally; the widths kept per project do not change. At 200% on a 1600-pixel window the Surface had been 0 pixels wide. Narrow tabs, segmented choices and the composer's selection chip end in an ellipsis | Found by running `h-library` and `d-daily` at 200%; the smallest rule that keeps every column and the Surface usable; W5.3 may replace it with a docking API |
| 2026-09-30 | **C-50, the Operator's decision:** the factory loop, a bounded cross-stage phase (§1.5, §2.10, §6.5). Stages 7 and 8 become one stage, "Stages 7–8", with milestones M1–M4 and work items W6.9, W6.10, W7.1–W7.4 and W8.1–W8.4; S7.1 and S8.1 are replaced by the design of §4.14–§4.15; Stage 6 keeps the Assistant's items. Stage 5 stays in progress; its open items and Stage 6's wait | The Operator's direction in this session |
| 2026-09-30 | Locked core, under C-50 (language), named before it is built: **(1)** `enum def`, `enum` values and enumeration values as feature values, for an agent's mode and for decisions and statuses in designed systems; **(2)** the standard `dependency` relationship, so our own core validates the self-model (R-41) and the dependency check runs on this repository; **(3)** the built-in `Agents` library of §4.11 (C-42) with `AgentOutput`, so the runtime reads confidence by identity; **(4)** expressions (KerML's literals, `null`, feature references and chains, unary `-` and `not`, `* / % + -`, comparisons, `== !=`, `and or xor implies`, `if ? else`, `new T(name = value)`, parentheses) as feature values, guards, effects and checks; references inside expressions are references like any other: linked by identity, printed by a name that leads back, unbound by name when their target is deleted; **(5)** the behaviour subset: `exhibit state`, `state`, `entry` and `exit` actions, `entry; then S;`, `transition [name first] S accept [x : T via p | after d] if g do effect then T;`, and the action nodes `send e via p`, `assign x := e`, `if e { } else { }`, `accept x : T via p`, `accept after d` and composite `action { }` run in written order; **(6)** scenarios as `verification def` with `subject`, `objective { verify r; }`, steps and `assert constraint`; **(7)** the built-in `Scenarios` library (`Outcome`, `StandIn`). New element kinds and fields are added to the tree; no existing construct, rule, meaning or identity changes; unsupported forms stay unsupported and are reported | Scenario I needs executable, checkable behaviour in standard terms (§4.14); C-50 authorises the minimum additive extensions |
| 2026-09-30 | Locked core, under C-50 (System State operations): `Property` gains variants for the new fields (an expression value, a guard, a trigger, a port for `via`, a transition's source and target, a state action's place); `Operation` is unchanged. Deleting an element unbinds references inside expressions exactly as other references | Model-facing edits of behaviour and scenarios go through the one typed change path, with locks, validation, history and undo |
| 2026-09-30 | Locked core, under C-50 (persistence): one optional file, `model/links.json` (§4.5 item 7), saved and committed by History with the model files; nothing else in the format changes | Implementation links must be versioned with the model and linked by identity (§4.15) |
| 2026-09-30 | Self-model under C-50: parts `Simulation` (crate `agq-simulation`), `Implementation` (renamed from `ImplementationLinks`; crate `agq-implementation`) and `Execution` (crate `agq-execution`), with the dependencies of §4.6; Q-20 decided: Simulation reaches providers only through a model client it is given | §8.1 rules 5 and 6: a distinct responsibility each, and one place that decides what may run |
| 2026-09-30 | Product terms under C-50 (§8.4, §9): run, trace, check, execution mode, implementation task, job, trusted-local execution; stand-in widened from an agent's model to any part a scenario replaces | A new term each where no standard one fits; "scenario", "simulation", "implementation link", "drift", "stand-in" and "recording" already exist |
| 2026-10-01 | Stages 7–8, built in one session (pending the Operator): the Studio's Scenarios tab and Run panel (modes, F5 and Shift+F5, verdicts apart from completion, a trace to step and play back with `[`, `]` and `\`, marks on the Surface only for a current result), scenarios written through controls, and the Inspector's Behaviour, Stand-in, Agent, Evidence and Implementation sections; what a scenario sets up (its stand-ins) is shown with the scenario, not as architecture cards | Scenario I's I2 and the Operator never writing SysML (C-4); stand-ins are test fixtures, not parts of the system |
| 2026-10-01 | The Assistant writes behaviour and scenarios with `apply_changes` (new kinds and fields: expression, via, after, guard, exhibit, initial, trigger, effect, subject, verifies, features) rather than a separate scenario tool; `inspect_behaviour`, `list_scenarios`, `run_scenario`, `stop_run`, `read_run`, `read_code_links`, `check_implementation` and `propose_implementation` are carried out by the Studio with its own services; a live evaluation is the Operator's to start, refused by the schema and again when prepared | One change path for every model edit (§1.3: generalise); the Assistant is a first-class user of the same services; spending money is the Operator's decision |
| 2026-10-01 | The supervised implementation loop: a task's brief comes from the model; the worker is the Assistant's loop with code tools in one git worktree per task; repair is bounded (6 rounds; stop after 3 without fewer failures); the Studio verifies the worktree itself and never takes the worker's word; the Operator reviews the patch, integration is refused if the repository moved, proposed links are applied only then; a wrong contract goes back to the Operator; one task at a time; the model folder is protected when the code shares the project's folder; a task continues in the same worktree without making it again | §4.15 and I4, I6, I8 with the smallest set of parts; bounded, reviewable autonomy |
| 2026-10-01 | Module boundaries allow a part def to use the definitions of its own parts, besides `dependency` | A whole composes its parts; without it no composing module could pass. From the model's structure, never from names |
| 2026-10-01 | An implementation run records the code once its harness is built | The build writes files (Cargo's lock file), which made a fresh result outdated at once |
| 2026-10-01 | Live evaluation: the model is shown a flat answer template (field, what it may hold) built from the output item, never the item's schema; a `fast` agent asks for the lowest effort its model lists; a live sample may run for 5 minutes (a model run, 10 seconds); the provider of an agent's model is the one whose own table knows the model id (§8.7) | Found in the first live runs: the model copied the schema's `fields` array (every answer invalid) and samples were cut at 10 seconds |
| 2026-10-01 | A recording's key sorts the request's keys itself | In the whole workspace's build another crate turns on `serde_json`'s `preserve_order`, so the key depended on the build that made it |
| 2026-10-01 | Library: `Resilience::RetryingWorker` and `Moderation` (an agent with a deterministic fallback) carry state machines and scenarios; using a block brings the scenarios whose subject it types; copies and plans remap every reference, expressions and `via` included (a fix); the curated library's cap rises from 40 to 60 blocks | §4.13 and C-50: blocks carry behaviour where it earns its place; each behaviour block needs its own items and ports |
| 2026-10-01 | Samples: "Start from the URL shortener" offers "With AI screening" (Scenario I's model) and "And its code" (its Rust code beside the project, committed and linked, with its harness), from one fixture shared with the proof test | The Operator can walk Scenario I, code included, without setting anything up |
| 2026-10-01 | Dialogs never grow taller than the window: their body scrolls and the buttons stay in view; "Create building block from selection" asks for its names before its lists | Found at 200% UI scale, where the trusted-local dialog's buttons fell below a 1000-pixel window |
| 2026-10-01 | Finding, for the Operator: `LinkScreening`'s `maxLatencyMs = 500` (fixed in §2.10) is not met by `deepseek-flash` from the reference machine (answers took 0.9–5 s, median about 1.8 s), so in a live evaluation the fallback decides every case and the benign cases fail; with 5000 ms, abuse was blocked 5/5 and benign links allowed 4/5. Not changed: §2.10 is the Operator's | A live evaluation's purpose: evidence about the agent's contract, reported as it is |
| 2026-10-01 | **C-51, the Operator's direction:** Stage 10 (Scenario C, §2.8) brought forward (§6.6); its gates A–E are the Operator's. Stage 5's and Stage 6's open items and Stages 7–8's acceptance wait. The organising principle is added to §1.5 and a non-goal to §1.6 | The Operator's instruction in this session |
| 2026-10-01 | §5.6: the foundation review at `f3d0dae2` (read-only), with the corrections by priority; its P1 items are W10.1. Confirmed defect: a task's verification counted a check only when it failed, so unrun, unsupported, inconclusive and unconfigured checks read as success, and integration was not gated on it | C-51 asks for the review before implementation |
| 2026-10-01 | §4.10's tool table corrected to the code (`run_scenario`, `check_implementation`, the worker's tools; scenarios are written with `apply_changes`) | One truth per topic: the table had kept the design names of C-50 |
| 2026-10-01 | Decided in this session under C-51, pending the Operator: the self-model moves to `model/` at the repository root, so the repository is an ordinary project and "Develop Agentique" opens it (§4.6); `models/` keeps the examples; the architecture check reads `model/` and ignores constructs it does not need, as R-41 said | One truth: the real self-model is the project's model, not a copy |
| 2026-10-01 | Decided in this session under C-51, pending the Operator: the part `ClaudeAgentRuntime`, implemented by the TypeScript companion in the new top-level folder `claude-agent/`; Node.js 22.6 or later is needed for that runtime only (an exception to R-17's "minimise Node", which still holds for everything else); the SDK and its Claude Code binary are pinned exactly by the lock file; protocol 1 over standard input and output | §4.7: the official SDK is TypeScript or Python; a process boundary keeps the Studio in Rust and the SDK's versions in one place |
| 2026-10-01 | Exception recorded under C-51 (§4.7, §8.7 rule 4): for the Claude Agent runtime only, the Anthropic key leaves the credential store into that process's environment, from which the SDK sends it to Anthropic; nothing else of the Studio's environment is passed | The SDK authenticates itself; the key still goes only to its own provider |
| 2026-10-01 | Decided in this session under C-51, pending the Operator: the part `Launcher` with the crate `agq-launcher`, depending on no other part, and `dependency from Studio to Launcher` (§4.6); builds in `%LOCALAPPDATA%\Agentique\builds\` (§4.9) | A bad build must not destroy the means of fixing it; local data, not roaming, for executables |
| 2026-10-01 | Decided in this session under C-51, pending the Operator: a project's required checks live in its app data (`checks.json`, §4.9), edited by the Operator; a task fixes its required set at approval; a verification passes only when every required check passed (§4.15) | A worker's working copy can never change the rules that judge it |
| 2026-10-01 | Decided in this session under C-51, pending the Operator: locks extend to integration review: a patch that changes code linked to a locked part asks explicitly; Agentique's safeguards (Execution, the verification in Implementation, `ClaudeAgentRuntime`, `Launcher`) are locked in its self-model | Reuses the lock (C-11) instead of a new kind of protection |
| 2026-10-01 | Decided in this session under C-51, pending the Operator: the required checks derived from links are required only where they can run (dependency boundaries with module links, scenarios with a harness); what is not configured is listed at approval as not checked and never counts as a pass (§4.15) | Requiring an unconfigurable check would block every task without a harness; hiding it would repeat the defect W10.1 fixed |
| 2026-10-01 | Decided in this session under C-51, pending the Operator: a task's worker reads and changes its worktree's copy of the model with the Assistant's model tools (same schemas, System State operations, locked elements refused); integration checks the task's model again and reads the accepted model afterwards; `Project::locks_at`, a read of the locks at a commit beside `tree_at`, is added to the System State for that check (no operation or format changes, R-16) | Self-hosted development changes the architecture with the code; the model changes stay proposed until the Operator integrates them |
| 2026-10-01 | An independent review of this session's safeguards found eleven defects, all fixed (seven with a regression test of their own): a lock set since the last checkpoint was invisible to tasks (a task with the model in the repository now needs a checkpoint first, and integration checks the current locks); integration's checkout matched paths as patterns; links were saved through a stale model after a model change; `links.json` changes in a task commit were not refused; a linked test passed on the first line of its name; safeguard code outside the locked crates was not linked to a locked part; a stopped runtime could still run a call and had no deadline; the launcher started a build without a manifest; the backup preceded the session save and left out My Library; verification ran in the worktree, not a clean checkout of the task commit; a failed worktree left a branch | §4.15's guarantees as stated; found by reading the code, not by a failure in use |
| 2026-10-01 | The conversation and a running task show their phase (inspecting the model, reading the code, changing the model, proposing a task, running checks, editing the working copy, waiting for you, writing the answer), from the tool being called (`agq_assistant::phase`) | C-51 asks for a visible phase; one mapping for both |
| 2026-10-03 | CI's baseline restored at the Operator's request (the System One plan, PR 0a): the agent worktree PR #86 committed as a gitlink (`.claude/worktrees/…`, no `.gitmodules`) leaves the index and agent worktrees are ignored, so checkout's credential cleanup works again; and `claude_agent.rs`'s `hidden`, code linked to the locked `ClaudeAgentRuntime`, no longer declares a variable mutable that only Windows changes (clippy denied it on Linux). No behaviour changes on any host | CI on `main` at `219598b6` skipped every check after the checkout failed; behind it, the Linux lint run failed too (§8.3: CI is green on a healthy tree) |
| 2026-10-03 | A locked safeguard changed at the Operator's explicit request (the System One plan, PR 0b): Execution's `Scope::resolve` reads a path the same way on every host (`/` and `\` both separate), so rooted, UNC and device forms (`\\server\share`, `\\?\`, `\\.\`) and `..` written with backslashes are refused on Linux as on Windows; relative paths in either separator, case-insensitive protection and the refusal of links leading out of the scope are unchanged, now with tests for both hosts' forms and for a link (a junction on Windows) | CI at `f3d0dae2` failed on Linux: `\\server\share\x` was one plain name there; a safeguard must not depend on the host |
| 2026-10-03 | **C-52, the Operator's direction:** the System One investigation's plan for Scenario I's typed decisions (updated 2026-10-03 after PR #86) is brought forward beside Stage 10 (§6.6, §7.1); its steps 0a, 0b, 1–5 and U are built in dependency order, each reviewable on its own; progress and the coverage checklist in `docs/stages.md`. Paid inference, live evaluations, production activation and acceptance stay the Operator's separate decisions | The Operator's instruction in this session, which explicitly reprioritises Scenario I's implementation alongside Stage 10 |
| 2026-10-03 | Locked core, under C-52 (persistence), named before it is built: optional fields in the existing run records only, no new file, store or evidence format. `AgentRequest.binding` (which provider and model answer, the adapter and mapping with their revisions, the canonical question, the input projection and the adapter's policy; strings, JSON and digests, never a provider type) is part of the recording key; `Recording.evidence` (the model that answered, its raw estimates, usage with its completeness, the request id, attempts); `Provenance.binding` (the binding's digest, on live and replay results); `LiveSummary.calls` and `LiveSummary.unknownCost`; the stop reason `budget-exhausted`. Absent fields keep legacy parsing and canonical bytes; a recording's key is recomputed when it is read; a legacy recording never answers a bound request, and a live or replay result without a binding is outdated, never current. The runs format stays 1 if, and only if, the previous build's reader is shown to ignore the new fields, never to match a bound recording and to skip a result it cannot read, so returning to it is safe and Stage 10's data-format check stays unchanged; otherwise the format changes and adoption is blocked until data rollback is supported. Thresholds and deadlines are applied to a recorded answer on replay and stay identified by the model digest, not the key | Exact replay and stale-result safety (A7); R-16 asks for an explicit decision; the smallest change that identifies what answered without a second store |
| 2026-10-03 | C-34's condition is met: `rig-typesafeai` 0.43.0 was released on 2026-09-30. The migration is its own pull request (C-52, step U); the five-task evaluation on every Assistant provider (§8.7 rule 5) waits for the Operator's consent. Until it is merged, the thin Jev client stays, corrected in place: a bounded exception to C-34 | rig's release changes the provider architecture; it must not ride along with the correctness repair |
| 2026-10-03 | Self-model under C-52: Providers owns typed decisions (complete bounded replies checked against the request, usage that says when it is unknown, one deadline, cancellation) and the capability table says which models chat and which decide; Simulation gives a live client each call's deadline and carries the Studio's binding; Studio → Providers also covers a live evaluation's model client. No new part, crate or dependency | §8.1 rule 5: changes across parts start in the self-model |
| 2026-10-03 | `docs/stages.md`, Stage 10: "uncommitted on the branch" corrected to merged in #86 (merging is not acceptance; gates A–E stay the Operator's) | One truth: the record had kept the development branch's wording |
| 2026-10-03 | The C-52 persistence change as built (step 3): as named, plus `LiveSummary.knownCostUsd` (when a call's cost is unknown the total is left out, so an older build shows the cost as unknown rather than a smaller total), `LiveSummary.calls` and `unknownCost`; a provider failure, a used-up allowance or a stop ends a live evaluation. The previous build's reader (`main` at `219598b6`) was run on data this build wrote (Linux, outside the repository): it read the four bound recordings without a problem and matched none of them by its own keys; it listed two of three results and skipped the one stopped as `budget-exhausted`; it showed the live cost as unknown. So the runs format stays 1 and the data-format check of Stage 10's adoption is unchanged. One limit on returning to that build: its own freshness ignores bindings, so it shows a bound live or replay result as current while the model is unchanged, as it did for every live result | §7.6 (C-52 persistence): the format stays only if the old reader is shown to ignore, never match and safely skip; it was, with this one weaker reading named |
| 2026-10-03 | Under C-52, the screening evaluation's definitions (the investigation's §7): the runner is an ignored test of the Studio (`screening_evaluation`), beside the existing live test, and the report is `tools/screening_report.py` (standard library only, tested on synthetic observations; CI now runs every `tools/test_*.py`). Observations and reports are written outside the repository and never committed (§8.3); the per-call observation lines are the runner's output, not a stored format of Agentique. The §7.7 thresholds are printed as PROPOSED, for the Operator to ratify; no labelled data ships and nothing paid was run | §8.1 rule 6: a reason for a new tool; the live evaluation stays the Studio's, and analysis (exact bounds, bootstraps, calibration) is where Python earns its place, as the investigation says |
| 2026-10-03 | rig upgraded from 0.42.0 to 0.43.0 (C-52 step U; C-34; §8.7 rule 5), its own pull request: providers built from configurations with the key given explicitly (rig reads no environment), one erased model type for every provider, `ProviderError` mapped to the same plain errors, usage taken apart again (0.43 counts cache reads and writes inside the input), reasoning sealed to the provider it is sent to (the Assistant already sends a model only its own reasoning, so no conversation format change), Anthropic's fallback adapter rebuilt as a rig transport over rig's own frames. Behaviour that changed, pending the Operator: tool input is no longer streamed on any provider (the capability table and §4.8 say so; the tool card shows "Preparing change…" until the call closes); a tool call whose input is not JSON ends rig's stream, which the Anthropic adapter repairs by handing the raw text as a JSON string, while on the other wires the reply ends at that call with the call kept and its usage lost. The five-task evaluation on every Assistant provider (`RIG_UPGRADE` in the evaluation set) is prepared and was not run: it is paid and waits for the Operator's consent and keys | rig released `rig-typesafeai` 0.43 and requires rig-core 0.43; keeping the Assistant's behaviour where rig no longer offers it, without a fork |
| 2026-10-03 | Decided in this session under C-52, pending the Operator: Jev stays on the thin client; `rig-typesafeai` 0.43.0 (evaluated from its released source and run against local servers) is not adopted. It validates the same distributions with the same tolerances the thin client now uses, but it accepts a key named twice inside `probabilities` and `legend` (the last wins), never compares the returned model with the pinned one, has no body limit, timeout or retry, loses the request id and usage when validation fails, sends a blank key, and is labelled experimental; adopting it would keep all of the thin client's transport and add a second parse. C-34's "until rig releases it" is therefore met by an evaluation, and the thin client remains a bounded exception, to be revisited when `rig-typesafeai` covers those points | C-34 asks for rig everywhere; §8.1 rule 3 and the investigation ask not to keep two layers that do one job |
| 2026-10-03 | **C-53, the Operator's direction:** Agentique improves itself. Stage 11 (§6.8) with Scenario J (§2.11); §1.5 gains its paragraph; the §1.6 rows on an autonomous factory, spending limits, supervised implementation work and self-improvement are edited in place; §4.2 gains objectives and the permission policy; §4.7's Claude Agent runtime gets the SDK's tools under that policy, the project's configuration and Anthropic-compatible endpoints; §4.15's integration, pushing, adoption and launcher rules gain their objective forms; §4.16 describes the Orchestrator, the control interface, typed decisions in operation and the lifecycle; C-5, C-35 and C-37 are amended in place; Stage 10's "Waits" no longer holds the Orchestrator or pushing within an objective | The Operator's instruction in this session supersedes the supervised-only rules explicitly |
| 2026-10-03 | Locked parts under C-53, named before they are built: **(1) `ClaudeAgentRuntime`**: the SDK's own tools stay on under the permission policy the Studio sends (read and write roots, protected paths with the model files always among them, refused commands, network, push and pull requests, extra MCP servers), enforced in the companion's one pre-tool hook, with undecided calls sent to the Studio; the project's settings are loaded (`settingSources: ["project"]`), never the machine's user settings or auto memory; the environment is the Studio's minus anything that looks like a key, token or secret, and the SDK keeps the model's key out of the session's commands, hooks and MCP servers (`CLAUDE_CODE_SUBPROCESS_ENV_SCRUB`); protocol 2 adds the policy, setting sources, subagent definitions and the model endpoint to `start`, and permission requests, queued messages, the pause gate and task, compaction and status events as messages; **(2) `Launcher`**: a supervising mode that stays the Studio's parent, starts the build a handover names, restarts a crashed Studio once and then falls back, and records a build as last known good only after its check after adoption; **(3) `Execution`**: unchanged in code; in a development session the SDK's own tools carry out file and command side effects under the policy, while verification, builds, integration and the Orchestrator's pushes and pull requests (exact commands on Execution's existing allow-list, no new operation) still go through Execution; **(4) persistence**: the project's format is unchanged; the Orchestrator's records are new app data (`objectives/<id>/objective.json` and `journal.jsonl`), listed among the data formats a build reports, so adoption compares them | R-16 and §8.1 rule 4 ask for an explicit decision; C-53 authorises these changes when documented, reviewed, tested and recoverable |
| 2026-10-03 | Self-model under C-53: the part `Orchestrator` (its crate `agq-orchestrator` is added to the model with the crate) owns objectives, cycles, their records, budgets and gates; it depends on Assistant (agent sessions), Execution (git, commands, worktrees), Implementation (required checks and verification), Providers (typed decisions, prices) and Launcher (builds and adoption), and the Studio depends on it; the Studio gains the control interface (port `control`); the requirement `GatesDecide` | §8.1 rules 5 and 6: one distinct responsibility (running objectives) that no existing part has, testable without a window |
| 2026-10-03 | The control interface under C-53, after its review: what is the Operator's own (approval dialogs and the Operator's own questions, Settings, the Conversation, locking, undo, appearance, the agents chip) is refused to agents where each effect happens, so no route (keys, focus and Space, the palette) reaches it; a change an agent's action makes is the Assistant's (`Actor::Assistant`, no new actor, so System State and History are unchanged) with the agent named; an action must name the observation it rests on, and the selection is part of the screen's shape; a supervised handover names the session and the project, and the Studio stops its work first | §3 roles and §1.6 (a visible record): an agent never gives the Operator's approvals or speaks as the Operator |
| 2026-10-03 | Product terms under C-53 (§8.4, §9): objective, cycle, permission policy, control interface, observation, typed decision; agent roles are named lead, implementer, reviewer and evaluator; a cycle's test instance is the existing "test instance" | A term each where no plain one existed; Orchestrator was already a product noun |
| 2026-10-03 | Under C-53: the reference machine has no Anthropic key, so the Claude Agent runtime is proven live through DeepSeek's documented Anthropic-compatible endpoint with the configured DeepSeek key (`deepseek-v4-pro`, `deepseek-flash`); a claude.ai or Claude subscription login is not used, as Anthropic's terms for the SDK require. A spike on 2026-10-03 ran the pinned SDK this way with its built-in tools, the project's `CLAUDE.md` and a pre-tool hook (4 model calls, 9 s); the SDK's own cost estimate assumes Claude's prices, so Agentique costs sessions itself | Live capability within existing authorisation; §8.7 rule 4's exception follows the key to its own provider's endpoint |
| 2026-10-04 | The Orchestrator under C-53, after its review: a criterion's command is a test run (cargo test, node --test, python -m unittest) that must fail on the base and run at least one test after; only one commit holding the reviewed tree is pushed per review, its message and the pull request scanned for keys; the code of locked parts is gated through the links (the Orchestrator's crate is linked to its part), Agentique's safeguards and agent configuration at any depth only when the objective names them; the baseline guard lists each weakened assertion, threshold or test; the lead and the reviewer run no commands, and a worktree session is refused the commands that move the repository's shared state (stash pop and drop, branch, update-ref, tags, local config, worktrees); test instances get no keys from the Studio's environment; usage without a price counts at a high one; a merge is idempotent; a build that does not take over ends the objective | §4.16's gates hold against an agent that writes its own criteria and code; commands an implementer runs stay unconfined (as any build is), so GitHub branch protection on the default branch is recommended as the backstop |
| 2026-10-04 | Typed decisions under C-53 (W11.6), after their review: a dialog in the Orchestrator's way is decided by rule (cancel it; an approval waits); Jev, the reasoning model and Jev escalating are compared live on 33 situations from the Studio's real dialogs and on a live application-control workflow (`docs/stages.md`); the comparison's reasoning model is `deepseek-v4-pro` through DeepSeek, fixed in `Decider::default` rather than taken from Settings. | §4.16 puts a known answer in deterministic code, and the measurements show no model does better there; DeepSeek is the only reasoning provider with a key on the reference machine. The self-model's Orchestrator doc ("each bounded choice in operation by a typed decision") is read with §4.16's rule for known answers; the locked part is unchanged. |
| 2026-10-04 | W11.7 under C-53, run live with an AI agent standing in for the Operator: after the bootstrap (and #103, a bootstrap fix made before the proof: an installed build did not know its repository), Agentique merged and adopted three of its own improvements (#104, #105, #106) from objectives entered in its window, each started in the build the previous one adopted or recovered to; interruption and resume, a failing gate repaired, a stale action and a failed launch with recovery were shown on purpose; what was not done (the Windows journeys and the reference run on the final `main`) and what the proof found are listed in `docs/stages.md` | The record of the run; the gate needs the Operator's own run and acceptance (§6.8, §8.3) |
| 2026-10-04 | **C-54, the Operator's direction:** Agentique tests and improves itself. Stage 12 (§6.9) with Scenario K (§2.12); §1.5 gains its paragraph; the §1.6 row on a multi-agent factory gains delegation within one objective; §4.16 gains models per role, credentials, exploration, testing knowledge, delegation, control ownership and observer mode, and the added evidence, gates and bounds; the glossary gains exploration, finding, testing knowledge, child objective and observer mode, each a concept no existing term names (exploration is not a scenario run, a finding is not a check's verdict, testing knowledge is not a run result, a child objective is not a subagent, observer mode is a speed of the visible agents) | The Operator's instruction in this session; Stage 11's loop is complete but is fed objectives from outside and judges command-only changes without the GUI |
| 2026-10-04 | Locked parts under C-54, named before they are built: **(1) `Orchestrator`**: models per role, resolved by the Studio before an objective starts, recorded with their reasons and used for every session of the role; the phases Explore and Reproduce before Propose when an objective explores; the criterion kind `replay` (a reproduced finding's steps and check, run on the base build, where it must fail, and on the change); evidence on the base as §4.16 states; evaluation of user-facing changes whatever the other criteria; failure identity by criterion and verdict; budgets for attempts, time, steps and model calls per role; child objectives within the parent's permissions and budgets, at most two deep; a cycle's worktrees removed when it ends; one build at a time; merged branches deleted; the escalation and decision models taken from Settings (amending the W11.6 entry's fixed `Decider::default`). **(2) `ClaudeAgentRuntime`**: the credential source the SDK reports is checked against the credential the Studio gave before the first model call, and a session that would use another stops with an `auth` error; `accountInfo()` and the source are reported in `init`; a status probe runs the bundled `claude auth status` with the Operator's configuration folder to report whether a local Claude login exists and of which kind, never its token, email or organisation; effort, the subagents' model and a spend ceiling (`maxBudgetUsd`) are passed per session. **(3) The Studio's rules for agents**: the Objectives panel's delegation field of a running objective accepts that objective's lead's directive, entered by the Orchestrator through the control interface and recorded as the lead's; in a test instance (started as one by the Orchestrator, holding no work of the Operator's), agents may undo and redo; a window is held by one agent at a time; every other Operator-own rule stands. **(4) Persistence**: the project's format is unchanged; the testing knowledge (`testing/<project>/knowledge.json`) and an objective's activity (`objectives/<id>/activity.jsonl`) are new files among the Orchestrator's records, covered by its existing `objectives` data format (1), which a previous build reads without touching files it does not know; `objective.json` gains optional fields only, so that format stays 1 if, and only if, the previous build's reader is shown to ignore them, otherwise it changes and adoption is blocked until data rollback is supported. The launcher and Execution are unchanged | R-16 and §8.1 rule 4 ask for an explicit decision; C-54 authorises these changes when documented, reviewed, tested and recoverable |
| 2026-10-04 | Under C-54, credentials as found on the reference machine: the Operator's machine has a claude.ai login (Claude Max) and no Anthropic API key, Console profile or cloud-provider credentials; Agentique does not use the claude.ai login (the SDK's documentation requires Anthropic's prior approval for products offering it [108]), so the Claude models of the role defaults cannot be tried live here, and each role falls back explicitly to DeepSeek's models with the reason shown. The capability table's `claude-opus-5-5` cache-read price is to be corrected from $0.40 to $0.20 and `claude-sonnet-5-5` added in W12.3 [109] | §4.16 credentials; report a provider limit as it is, never as a pass |
| 2026-10-04 | **C-54 amended by the Operator:** the Operator gave Agentique their own Claude subscription token (`CLAUDE_CODE_OAUTH_TOKEN`, from the documented `claude setup-token` [108]) for their own work on this machine. Agentique uses it only in the Claude Agent runtime, as one explicit credential beside the API key, checked against the source the SDK reports before the first prompt, kept out of the session's subprocesses and never logged; it still never offers a claude.ai login, never reads the CLI's stored login and never extracts a credential. A live check through the pinned SDK 0.3.287 the same day: `accountInfo()` answers before any prompt with `apiProvider: firstParty` and `tokenSource: CLAUDE_CODE_OAUTH_TOKEN`, the init message says `apiKeySource: none`, and `claude-opus-5-5` and `claude-sonnet-5-5` answered. Roles that call Claude directly (escalation, the Assistant's own loop) still need an API key and take their fallbacks here | The Operator's instruction in this session; the SDK's rule is about products offering claude.ai login to others, and the Operator decides how their own subscription is used |
| 2026-10-04 | **C-54 clarified by the Operator** ("one window, role models"): the Conversation is the one window for intent, agent communication, observation and steering, and the Objectives panel a dashboard of the same objectives, both on the same records and commands; the Assistant is the entry point and answers ordinary requests on its role's model, while an objective started from the Conversation (or the panel) runs the lead and the specialists on theirs, with no agent forwarding messages; directives are first-class records (author, recipient, parent objective, scope, status, result) created only by a tool the Orchestrator validates, streamed into the objective's thread, with scoped handoffs and separate sessions; an objective continues by itself through cycles, adoptions and supported restarts within its budgets; agents genuinely operate the Conversation in test instances (`--test-instance`), never in the Operator's window. §4.16 ("The Operator's own", visible agents, the Conversation, directives and delegation, durable work), §3.6, Scenario K (K1, K4, K8), Stage 12's work items, C-54 (§7.1), the glossary (Conversation, Objectives panel, directive, thread), `AGENTS.md` and the self-model follow. This amends the locked-parts row above: in (3), the Objectives panel's delegation field is not built, and agents may operate the Conversation in a test instance; in (4), the objective's thread (`objectives/<id>/thread.jsonl`) replaces the activity file, under the same `objectives` data format | The Operator's instruction in this session; it replaces the delegation field typed into the Objectives panel with structured directives the Conversation shows |
| 2026-10-04 | Decided in this session under C-54, pending the Operator: a test instance of a merged build used for exploration may be given the explorer's provider key for its own Assistant, so self-testing of the Conversation reaches real answers within the objective's spend budget; a test instance of an unreviewed build (evaluation) still receives no credential (W11.5) and its Assistant runs on the scripted stand-in the journeys use, so the Conversation's own behaviour is testable there, while a criterion that needs a real model is not run, never a pass. An objective continues by itself after an adoption or a crash the launcher recovers; one interrupted because the Operator closed Agentique waits for the Operator's Continue, as in W11.7, and a Pause is kept across restarts | Self-testing the Conversation needs a model to answer; merged code has been reviewed, unreviewed code has not; closing Agentique is the Operator's way to stop spending |
| 2026-10-04 | W12.6 under C-54, persistence: beside a project's conversation file the Studio keeps `conversation.objectives.json` (when each entry was added, and the objectives started while that conversation was open), so an objective's thread interleaves in time and shows only in the conversation it was started from; it is covered by the existing `conversation` data format (2), whose file is unchanged, and a previous build never reads it. A test instance never runs an objective (it shows a recorded one), the Assistant's `propose_objective` only opens the start form, and a message to an ended objective is refused | One place for intent without changing the conversation's format; self-testing must never start real work |
| 2026-10-04 | W12.5 under C-54, as built: an objective continues by itself only after an adoption (its continuation) or a fallback to the last known good build (`--recovered-from`); a crash the launcher restarts in the same build passes no flag, and the launcher is locked, so such an objective waits for the Operator's Continue, as after a close (the Studio marks it interrupted as it closes). A reproduced finding a failed cycle left unfixed stays eligible, once it reproduces again on the new base, until the objective has tried it twice. Evidence on the base, as built: command criteria run with the checked commit's own new and changed test files, recorded with their blob ids and run again when an attempt's differ; only a failing test whose own output shows an assertion failing counts (not an `unwrap` or `expect` on nothing, a `TypeError`, a Python `ERROR`); an observation counts only when its own expectation fails (a setup action that failed, a broken connection or an instance that did not start is no evidence); only the frozen criteria and the replay count | The independent reviews of W12.5 (PR #115); §4.16 "Durable, continuing work" and "Evidence, gates and bounds" |
| 2026-10-09 | **C-55, the Operator's direction:** the purpose is recorded once, in §1.1, with the root-system and generalisation principles; §1.5 gains its paragraph; §4.3 gains the C-55 subset growth, the rule that a construct is complete on the whole path, and "the same constructs for Agentique and its users"; §4.14's claims become the nine of its table (parse, resolution, rules, calculation on the model, executable, completed, check passed, implementation agrees, evidence under stated assumptions), a `satisfy` declaration being a claim, not evidence; §4.16 gains the protection of `ROADMAP.md` and the purpose from cycles and "Alignment with the purpose"; Stage 13 (§6.10) folds W12.7 in as W13.7; the glossary gains approved baseline; `AGENTS.md` refers to §1.1 | The Operator's instruction in this session: autonomous development must serve the purpose, and Agentique's foundations must carry it |
| 2026-10-09 | Locked core under C-55, named before it is built (R-16). **Language core:** referential usages `ref part` and `ref item` (a flag on part and item usages; the kind, and so every identity locator, unchanged), with `composition-cycle` ignoring them, a referential value that must name a part or item usage of a fitting type, and a composite part bound to another part reported (stricter than the standard, a new deviation); `assume constraint` and `require constraint` in requirement definitions and usages, named or not, formal (an expression) or informal (a doc comment), and subrequirements, whose subject is the container's; `calc def` with `in` parameters and a result, invoked with named arguments, if Stage 13's scenarios need it. **System State operations:** the properties that create and edit them (the referential flag; constraints through the existing expression property). **Persistence format:** unchanged; the new constructs are SysML text, and the identity file keeps its format; an older build reads them as unsupported text, kept verbatim, so a project that uses them loses nothing when opened there, but shows them as unsupported until it is opened in a build that reads them | Composition versus reference, and requirements that can be evaluated, are what the scenarios of §6.10 ask; nothing else in the core changes |
| 2026-10-09 | Locked safeguard under C-55, named before it is built: **Orchestrator:** `submit_proposal` requires `serves` (requirements of the project's model), `benefit` and `complexity`, and checks `serves` and `parts` against the base's model; review lists changed-but-not-named and named-but-not-changed elements by identity and the cumulative change since the approved baseline (the tag `approved-baseline`, the Operator's only; without it, the objective's start, said so); a change to `ROADMAP.md` or to the purpose requirement fails the gates even when named; the lead records a finding's disposition (defect, wrong expectation, ambiguous requirement, unreliable reproduction) with `adjudicate_finding` before proposing it, only a defect is proposed, and wrong expectations are not offered again. Records: optional fields in `objective.json` (format 1) and the testing knowledge (format 1), read by the previous build | §4.16's alignment: the purpose constrains the loop through its existing records, requirements, locks and gates, not a second governance framework |
| 2026-10-09 | Self-model under C-55 (§8.1 rule 5): the requirement `purpose` (its definition `Purpose`, subject `Agentique`) refers to §1.1, added through `apply_changes` (W13.1) and locked on the Operator's C-55 instruction; no `satisfy` is declared until its obligations exist and the Requirements panel keeps a declaration apart from evidence (W13.3, W13.4); `apply_changes` also printed the model as Agentique saves it (its `//` notes, which are not model elements, are gone; `model/README.md` explains the order); the purpose's obligations as subrequirements, Simulation's evaluation of requirements on a modelled configuration (no new part, crate or dependency), the Orchestrator's behaviour and role agents, and the test instance as a referential `Agentique` follow in W13.3 and W13.4 | Changes that cut across parts start in the self-model |
| 2026-10-09 | The approved baseline starts at `f3f313b0`, the `main` the Operator reviewed before giving C-55 ("the last reviewed main was f3f313b"); the tag `approved-baseline` is set there when this record is merged, and only the Operator moves it (`git tag -f approved-baseline <commit>` and a push) | A cumulative review needs a human-approved reference; the Operator named this one |
| 2026-10-09 | `docs/stages.md`, Stage 12: the work-item table and the "built" notes said "not merged" for W12.2, W12.3, W12.5 and W12.6; all were merged (#110, #111, #113–#116) before `f3f313b0`. W12.7 had not started and is W13.7 | One truth: the record had kept the branches' wording |
| 2026-10-10 | Locked core under C-55, as built (W13.2): `ref part` and `ref item` are a `referential` flag on part and item usages (kind and identity locators unchanged; printed `ref part`, after direction and `abstract`), set by the System State's `Property::Referential`. One rule decides what is referential (SysML 7.6.3, `validateUsageIsReferential`): a usage referential itself (`ref`, no kind keyword, directed, an `end`, or owned by a package) whose redefined part and item usages are all referential too, since a redefinition has the values of what it redefines (KerML 8.3.3.3.8); so `:>> supply = bus;` binds a reference only when the inherited usage is itself referential, and a `ref` redefinition of a composite part is composite. Validation: `composition-cycle` skips references; a reference's value must name a part (for `ref item`, a part or item usage, or `new T(...)`) whose types specialise every type of the reference and that is not itself, directly or through other references (`wrong-value`); `ref part … = new T()` and a binding to a usage without a kind keyword that redefines no part are `unsupported`; a value-less redefinition keeps the binding it redefines, and a binding is never changed in a redefinition (KerML `validateFeatureValueOverriding`); a composite part or item bound to a part is `wrong-value`, judged by the part the binding reaches: a composite part of another owner by the standard's rule (SysML 7.6.3), anything else, such as a part of the same owner, by deviation 19 (stricter than the standard); whether a connection passes items inward is decided by what a reference is bound to (deviation 8). Model execution runs a bound reference as the instance it refers to (no copy: state, ports and stand-ins are shared); one not bound (the part it refers to is not identified in this model) has its ports only, so a connection through it compiles, a message reaching it stops with `missing-stand-in`, and a stand-in may answer for it; a bound reference with features of its own does not run; a `ref item` stays a value and cannot refer to a part. The persistence format is unchanged (an older build reads the new text as unsupported, kept verbatim). The model-slice digest includes the element's fields, so run results stored before this change show as outdated once | Composition versus reference (§6.10's first question): a shared element is referred to, never copied or counted twice, and a system can refer to another instance of its own kind |
| 2026-10-10 | Locked safeguard under C-55, as built (W13.5): `submit_proposal` requires `serves` (requirements of the project's model), `benefit` and `complexity`; `serves` and `parts` are resolved by identity in the base commit's model at submission, a failure refusing the proposal with the model's requirements listed, and the resolved ids recorded (a base without a model folder resolves nothing, a model with no requirement yet records `serves` as stated, each saying why; a model that cannot be read stops the cycle); the review receives, and the cycle records, "changed but not named" and "named but not changed" (model elements by identity, and parts whose linked code changed) and the cumulative change since the `approved-baseline` tag read with `git ls-remote` at the URL of `origin` recorded when the objective was created, never the local tag (else the objective's start, saying why), and the reviewer must answer `traceability` and `purpose`, without a score, and is told how the finding a change fixes was judged; in a project whose model declares a root purpose requirement (Agentique's does; a user's project without one is not governed by it), the gate "purpose and governance unchanged" fails a change to `ROADMAP.md` or to `Purpose`/`purpose` and everything they own, also moved or renamed, as the base, the objective's start and the approved baseline declare them, whatever the objective names, and goes to repair like any gate; models, links and their presence are read by the commits' trees through a clean checkout, never the one the checks ran in (the locked-element gates too); worktree sessions are refused `git tag` and changes to the remotes, and every session, the Operator's Conversation (working-copy sessions) included, is refused git aliases and includes given on the command line or in the environment (quoted too), `gh` releases, tag refs and GraphQL mutations, and `gh` without the network; these rules cannot close every way, so the approved baseline's real control is a GitHub tag ruleset on `approved-baseline`, recommended to the Operator; the recorded `origin` URL is kept without credentials; every chosen finding needs the disposition `defect`, recorded by the lead's `adjudicate_finding` in the testing knowledge and kept once its proposal is accepted, a finding's id is its place in the cycle and never changes, a wrong expectation is not offered again and its rediscovery is not new, an unreliable reproduction is offered again only when found on another build than the one it was judged on, an ambiguous requirement becomes a question for the Operator in the thread and is not reproduced again (reversing a disposition is not an Operator command yet), and a cycle whose findings are all judged otherwise ends as done with nothing to fix, its directives saying so. The model is read for these checks through `agq-assistant`'s `model_tools` (the Orchestrator does not depend on the language core). Records: optional fields in `objective.json` (with `origin`) and the testing knowledge (both format 1) | §4.16's alignment as built: the purpose constrains the loop through its records, requirements, locks and gates |
| 2026-10-10 | Locked part under C-55, as built (W13.4): the self-model's `Orchestrator` now owns its `Driver` (the phases of §4.16 as a state machine following `run.rs`), its role agents, their `RolePort` and results, `CycleChecks` (a cycle's checks: required checks, evidence on the base, gates), the replay items and `InstancePort`, `NothingToFix`, the reference `testInstance` (another Agentique, explored through its control interface), and the ports `jobs`, `launch`, `instances` and `observe`; the product gains `control`; its `control` port and the connection to `studio.control` are removed; `ObjectiveRequest` gains `mayMerge`, `mayAdopt` and `attempts`, `Directive` gains `refused`. Made through `apply_changes`, the Orchestrator's lock confirmed for each change on the Operator's C-55 instruction. Folded, and said so in the Driver's doc: Evaluate into Check, Try into the build's answer, the same failure twice into a round without fewer failures. Not linked to a test because none exercises them (15 of 34 transitions): a rejected review, a merge, its repair or permission, a build and its try, a build that does not take over as the Driver records it, a child at the maximum depth, the start and continuation of cycles | §8.1 rule 5; an independent review found the first version claimed behaviour the code does not have (it is redone from `run.rs`) |
| 2026-10-10 | Locked core under C-55, as built (W13.3): `assume constraint` and `require constraint` [name] { expression } in requirement definitions and usages, informal when only a doc comment (element kinds `AssumeConstraint`, `RequireConstraint`; no new field, System State property or persistence change; `assume x;`/`require x;` and short names on constraints stay unsupported; constraints and checks may be private); subrequirements and redefined requirement attributes; validity code `misplaced-constraint`. Simulation evaluates each `satisfy` on the modelled configuration of what satisfies it (built in the part or part def the satisfy is written in; members counted whatever their visibility; a `ref part` is the part it refers to, counted once, and an unbound one is not determined): holds, violated, assumptions not met (a subrequirement whose assumptions are not met does not apply), or not evaluable (stricter than the standard's implication, deviation 21; binary64 numbers, deviation 22; scenario runs still work attribute values out differently, deviation 23; a requirement that contains itself, an ambiguous subrequirement name, a subject bound twice or a satisfying feature of multiplicity other than one are not evaluable), with the values used and the digest of the slice read; nothing is stored. Implementation keeps the evidence ladder (declared, calculated, scenarios with freshness, an implementation run current only while the code is what it ran, linked tests); only a failed check or a stop by the model's own behaviour is a scenario failure, anything undecided is an inconclusive rung that outranks nothing; "holds" needs every applying calculation to hold (else "holds for k of n"). The Requirements panel and the Inspector show the ladder, the headline counting standings of requirement usages (subrequirements counted with their container), never a declaration as satisfaction; the Assistant's read-only `check_requirements` gives the same and says, headless, that kept results and links are not available | §4.14's claims kept apart for requirements; requirements that can be checked on a configuration |
| 2026-10-10 | Deviation 14 widened under C-55 (W13.6): from durations in milliseconds to every quantity, a plain number in the unit its attribute's documentation states; measurement expressions (`1200[g]`) stay unsupported and nothing is converted or dimension-checked. The second system, `models/inspection-charging/` (an inspection drone and its charging station, after the Operator's guide), states its units so | The drone's mass budget and bounded wait are calculations; the ISQ and SI libraries are not pinned (§6.10 "Waits") |
| 2026-10-10 | Self-model under C-55, as built (W13.4, the purpose's obligations): `Purpose` gains eight informal subrequirements (explicit architecture; executable or reported; claims kept apart; the same mechanisms for itself; the Operator keeps control; aligned evolution; the root system; generalise the mechanism), each naming the requirements that make it concrete or "none yet", made through `apply_changes` with the lock of `Purpose` confirmed on the Operator's C-55 instruction; `satisfy purpose by agentique` is declared now that the Requirements panel keeps a declaration apart from evidence, and a dogfood test asserts the purpose stands "only declared", never as holding | §1.1: the purpose's obligations held in the model it governs; the claims of §4.14 apply to it too |
| 2026-10-10 | Deviation 20 widened under C-55 (W13.6): a usage that subsets another usage takes its configuration and its redefinitions replace that usage's values (the upgraded drone states only what changed from the survey drone); the standard reads `:>` between usages as "is one of" and forbids overriding a bound value | Reuse of a configuration without copying it, recorded as Agentique's reading rather than given the standard's name |
| 2026-10-10 | **C-54 amended by the Operator** (the start of an objective, during W13.7: the start form's card could not be scrolled, so Start was out of reach, and the form asked for too much): an objective is started from its intent alone. What it does (whether it explores first, how many improvements, whether a reviewed change that passes every check is merged, and then built, tried and adopted) is inferred from the intent by a typed decision (Jev, the `decisions` role, escalating to the `escalation` role's model when unsure, as every typed decision does; the defaults when neither answers, said so) and shown before Start with who inferred it, where the Operator may change whether it explores, merges or adopts; the start form no longer asks for spend, time, attempts or exploration steps, nor lists each role's model (the thread still records each role's model as the objective starts, and a role without a model still stops the start with why). Spend and time are unlimited unless set (a child objective's are: the lead gives it a spend budget, and its time is an hour at most): an objective is bounded by its cycles, its attempts per cycle (4), its exploration steps (20 per run), each role's model calls, lack of progress and the Operator's Pause and Stop; costs are still counted and shown. Persistence, named before it is built and decided by the Operator (R-16; asked, "leave the limits out", over writing the largest number, which older builds would read): in `objective.json`, `budgets.usd` and `budgets.hours` become optional and are left out when unlimited, and what the intent was read as is a new optional field `inferred`. A build before this change cannot read such a record: its list (`Store::list`, and so `active()`) leaves it out, so it neither shows nor continues it and could start another objective beside it; and its check after adoption (`Store::readable`) refuses, so while such a record exists a build before this change cannot be adopted and the launcher stays on, or returns to, the last known good build. This departs from the condition of the C-54 locked-parts row (the format stays 1 only if the previous reader ignores the new fields); the `objectives` data format stays 1 by this decision. Builds from this change on read both kinds of record. The Assistant's `propose_objective` proposes an intent only (what it does is read from it). The Orchestrator does not open a pull request without the permission to merge: a reviewed change it may not merge stays on its branch and the cycle ends there, as the form says | The Operator's instruction in this session ("just explore if needed, unlimited budget, ... infer from intent", and that the inference is Jev's); the Operator still sees and may change the consequential choices before Start |
| 2026-10-10 | W13.7 repair, the Orchestrator (a locked part; this task's authorisation): a failure of the repository's checks is attributed before the implementer is sent back. Each failed job's log is read into its failing tests and their crates. A failure is elsewhere (a defect already on the base) only when every failing test is in a crate the change neither touches nor can affect by the workspace's dependencies (paths read without rename detection; anything outside the crates but prose counts as affecting every crate) and reads no files outside its own crate; a check that never reached a verdict (cancelled, or failed setting the job up) is the checks' own machinery; everything else is the change's. Either way the cycle stops with its reviewed change kept on its pull request (`Cycle.blocked`, with its `cause`). For a failure elsewhere, where the objective may push and merge, a repair cycle (`Cycle.repairs`: not one of the improvements, not repaired in turn, no delegation) proposes only the repair, and its implementer and reviewer are told its scope; after it merges, the blocked change is carried onto it only if the repair touches what failed and the change's patch reads unchanged (a clean three-way merge whose diff agrees but for index lines and hunk positions), pushed onto its pull request without forcing, and merged when the repository's checks pass on exactly that commit (a failure there is attributed again); the repair cycle then builds, tries and adopts that merge (`Cycle.carried`), and the carried cycle is adopted with it. A carry interrupted by closing Agentique, or paused by the host or the working copy (checks that reached no verdict on it among them), is taken again from the merge phase (each side effect once); Stop ends the objective, and a carried commit already pushed stays on the pull request, which the thread says. The objective record gains optional fields only (`blocked`, `repairs`, `carried`), which the previous build ignores when it reads; if a previous build saves the record (after a rollback), it drops them, counts a repair cycle as an improvement and loses the link between the cycles. Limits: the trial observes only the repair cycle's criteria, and the build's manifest lists the repair's local checks, not the carried change's; test targets after the first failing one did not run, which the thread says | The first proof attempt's #125 failed on a race in a test it does not touch, and the implementer was sent to repair what was not its own; a change's own failure, a failure elsewhere and the checks' own machinery are different findings |
| 2026-10-10 | Locked part under C-54, as built (W13.7 repair E1, what an exploration explores; the Operator authorised these repairs in this session): `submit_exploration` requires `project` (a folder holding a model's `.sysml` files at the base commit, as the brief lists them: not the pinned standards, test fixtures or a folder inside another) besides `goal`, and takes `scope`, `start` (a view's command, an element to select) and `vary` (other projects, only when the objective asks for several), each checked when submitted (names resolved in that project's model at the commit); a plan the lead's turn accepts is recorded on the objective at once, so a child delegated after it in the same turn explores it; `delegate` takes `project`, which it needs before the first plan, and a child never explores another project than the one it was given; a lead that plans nothing is asked once more and the cycle then ends (the alternation between the samples is gone); later plans, replays of fixed and known findings and the evaluation's exploration keep to the plan's projects; each exploration and replay checks the copy its instance opened (folder copied, files' digest, project shown) against the project at the build's commit before acting, a mismatch failing the exploration's cycle, diverging a replay and failing the evaluation's exploration. Records: optional fields only, formats unchanged (`objective.json` `target`; the testing knowledge's run `start`), so a previous build reads them; one that saves such a record drops `target`, and this build then explores again only after the lead plans again (an exploration already handed over ends its cycle with the reason). Self-model: the docs of `Orchestrator::Plan`, `Explored`, `Delegation` and `ReplayRequest`, through `apply_changes` with the Orchestrator's lock confirmed | The W13.7 proof: the lead planned Agentique's own model, but the Driver alternated the start, so two of four explorations opened the URL shortener and found nothing relevant |

### 7.7 The original requirements

The 20 requirements of the original specification (`Agentique-Specification-v0.1.html`,
kept as history) were reconciled in `REALIGNMENT.md` §6.5 (retained,
superseded or deferred, one line each). That reconciliation stands as recorded
on `main` at `6fc90b78`; its retained content is in this document (§4.2, §4.5,
§5.5, §8).

---

## 8. Rules against drift

### 8.1 Governing rules (`AGENTS.md` must state these)

1. **`ROADMAP.md` governs, and its purpose (§1.1, C-55) governs every
   change.** Every piece of work names the stage (§6) and the scenario step,
   decision (C-n) or recommendation (R-n) it serves, and, inside an
   objective, the requirement of the project's model it serves. If it serves
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
   `model/` first, through Agentique's own operations (the Studio, the
   Assistant's tools, or, for an agent without a window, the `apply_changes`
   example in `README.md`), never by editing its files. The dependency check
   (R-15) must stay green.
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
   `cargo test --workspace`, `python tools/check_architecture.py`, and the
   companion's tests for changes to `claude-agent/`) before
   handing work back, and report the actual results honestly, including
   failures. Changes to the Studio or the Surface also run the reference budget
   run (§8.6).
10. **Inside an objective** (C-53, §4.16): the model changes only through
    Agentique's tools; an agent works only in its cycle's worktree (file tools
    are held to it, and the change is checked path by path before merging);
    the agent configuration (`.claude/`, `CLAUDE.md`, `AGENTS.md`) changes only
    when the objective names it; besides the SDK's subagents inside its own
    session, work goes to another agent only as a child objective its lead
    delegates through Agentique, within the objective's permissions and
    budgets (C-54); the
    acceptance criteria and required checks stay frozen at the proposal; no
    test, check or budget is deleted, ignored or loosened to make it pass (a
    deliberate change is named, with its reason, for the reviewer); no
    force-push, no push to the default branch, and only the Orchestrator
    merges, when the gates pass. `ROADMAP.md` and the self-model's purpose
    requirement are never changed inside an objective, even when it names
    them (C-55): such a change is proposed to the Operator. A proposal names
    the requirements it serves, the elements it affects, its benefit, its
    evidence and its effect on root complexity; a finding is judged against
    the requirements before it is fixed (a wrong expectation is not a
    defect).

### 8.2 One truth per topic

| Topic | Document |
|---|---|
| Direction, decisions and stages | `ROADMAP.md` |
| Stage progress | `docs/stages.md` |
| Agentique's purpose | `ROADMAP.md` §1.1 |
| What Agentique does and how to run it | `README.md` |
| Architecture | `model/` (Agentique's own project model) |
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
- **Claims about systems built with Agentique stay apart** (C-50, C-55):
  the nine claims of §4.14's table, from "the text parses" to "evidence
  supports a requirement under stated assumptions". A result says which mode
  produced it and whether it is current; an outdated result is never shown as
  current evidence, and a live evaluation not run is never reported as passing.
  Deliberately introduced defects (in the model and in the code) show that a
  check detects what it claims to.
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
  Assistant, Orchestrator, System State, Settings, lock, scenario,
  simulation, implementation link, drift, agent, autonomy mode, Library,
  building block, My Library (C-49), run, trace, check, execution mode,
  stand-in, recording, implementation task, job, trusted-local execution
  (C-50), runtime, build, test instance, launcher (C-51), objective, cycle,
  permission policy, control interface, observation, typed decision
  (C-53), exploration, finding, testing knowledge, child objective,
  observer mode, directive, thread (C-54), approved baseline, disposition
  (C-55); the explorer joins the agent roles.
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
| **Panels** | Side panels for detail (Outline, Inspector, Requirements, Problems, History, the Run panel, the Objectives panel) |
| **Conversation** | Chat with the Assistant, and the one place where the Operator gives intent and follows and steers every objective's agents (C-54) |
| **Assistant** | The AI agent the Operator works with, with tools and skills; its sessions also do an objective's work for the Orchestrator |
| **Orchestrator** | The part that runs objectives: it directs the Assistant's agent sessions (lead, implementer, reviewer, evaluator, explorer) through cycles, while deterministic checks and review decide (C-53, C-54) |
| **System State** | The live, authoritative KerML/SysML description of the system being built |
| **Lock** | A mark on a part meaning it changes only with the Operator's confirmation, or within an objective that names it (C-53) |
| **Settings** | The Studio view for the Operator's choices: providers and keys, models, the Assistant's behaviour, appearance, keyboard, projects |
| **Provider** | A service that runs models: Anthropic, OpenAI, OpenRouter, DeepSeek; TypeSafe AI (Jev) for typed decisions of fast agents and the Orchestrator (C-53) |
| **Autonomy mode** | How much the Assistant asks before acting: ask before every change, ask on major decisions, or ask only on locks |
| **Plan card** | The Assistant's short plan for a task, shown in the Conversation and updated as steps finish |
| **Skill** | An instruction file (`SKILL.md`) the Assistant follows; built in or the Operator's own |
| **Note** | A short piece of information the Assistant proposed and the Operator approved, used in later conversations |
| **Compaction** | Summarising earlier conversation so the Assistant's context stays within limits |
| **Agent** | A part whose behaviour is produced by an AI model, in fast or deliberate mode; in the model, a part definition specialising `Agents::Agent`. The Assistant is Agentique's own agent. An agent's fast mode is unrelated to Anthropic's "fast mode", a faster serving option |
| **Fallback** | The deterministic part that takes over when an agent fails |
| **Stand-in** | A deterministic replacement, declared in a scenario, for an agent's model or for any part of the subject: it answers calls with a stated outcome (answer, timeout, invalid output, refusal, unreachable tool) instead of the real behaviour (C-50) |
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
| **Scenario** | A verification case: a subject, the requirements it verifies, stand-ins, steps and checks; it runs unchanged in every execution mode (C-50) |
| **Run** | One execution of a scenario in one execution mode against a fixed model snapshot (and, for implementation runs, a fixed code revision), with its trace, check verdicts and provenance |
| **Execution mode** | What a run actually exercised: model execution, recorded replay, implementation, live evaluation, or walkthrough (which verifies nothing) |
| **Trace** | The ordered, recorded events of a run: what happened, where, with which inputs and outputs, and why it stopped |
| **Check** | An `assert constraint` of a scenario, or an implementation check; its verdict is passed, failed, not run, unsupported, blocked or inconclusive, and it may be outdated |
| **Implementation task** | One approved piece of implementation work with a scope, a plan, a worktree, checks and a patch for review |
| **Job** | A long-running piece of work with side effects (an implementation task, an implementation run, a live evaluation), with a stable id, a state and a journal |
| **Trusted-local execution** | The per-project mode in which Agentique may run builds, tests and harnesses on this machine with the Operator's rights, because no sandbox is available |
| **Harness** | The small program in an implementation repository that lets the implementation runner drive the real code through a scenario's steps |
| **Implementation link** | A link from a model element to code, tests or a running service |
| **Drift** | A failing check on linked code, shown at the model elements it covers: a detected difference between the model and a linked implementation |
| **Library** | The Studio's collection of reusable definitions, shown as building blocks: the built-in blocks, the project's own definitions and My Library (§4.13) |
| **Building block** | A reusable definition as the Library shows it: atomic (one definition) or composite (a part definition with its own parts, ports and connections) |
| **My Library** | The Operator's own building blocks, kept in the app's local data for use in any project; using one copies it into the project |
| **Runtime** | What runs the Assistant's turns: Agentique's own loop over a provider, or the Claude Agent runtime, in which the Claude Agent SDK runs the loop and Agentique carries out its tools (C-51) |
| **Claude Agent runtime** | The companion process that runs the official Claude Agent SDK for the Assistant, with the SDK's tools under the session's permission policy and Agentique's tools beside them (C-51, C-53) |
| **Build** | A release build of Agentique from one commit, in its own folder with a manifest of what it was built from and after which checks |
| **Test instance** | A build started with its own app data and sessions, to try it without touching the running Studio's data or project; inside a cycle, also the debug build of the cycle's commit that the evaluator operates (C-53) |
| **Launcher** | The small program that starts the current build of Agentique and falls back to the last known good one |
| **Last known good build** | The most recent build that started and reported ready (after an adoption, a build reports ready only once its check after adoption passed, §4.16) |
| **Objective** | The Operator's intent for autonomous work, with budgets and the permissions it carries; the Orchestrator pursues it in cycles until it is met, a budget is used up or the Operator stops it (C-53) |
| **Cycle** | One improvement within an objective, from proposal with frozen acceptance criteria through implementation, checks, evaluation, review, merge, build and trial to adoption |
| **Permission policy** | What a Claude Agent runtime session may do: where it may read and write, protected paths, refused commands, network, pushing and pull requests; enforced in the companion, decided by the Studio |
| **Control interface** | The Studio's structured observation and actions, through which agents operate the real, visible application without computer vision |
| **Observation** | A text snapshot of the Studio's visible state (identity, screen, dialog, selection, controls, commands, tasks) that actions are checked against; not to be confused with a run's results, which are observations of a system under stated conditions |
| **Objectives panel** | The Studio's dashboard of objectives: budgets, phases, effective models and spend per role, child objectives, and Pause, Step, Resume and Stop; it starts and steers objectives with the same commands as the Conversation |
| **Typed decision** | One atomic question with explicit inputs and typed options answered by a fast decision model (Jev) under a deadline; uncertain or failed answers escalate (C-52, C-53) |
| **Exploration** | An explorer agent operating a test instance through the control interface toward a goal, choosing each action among the valid ones from what it observes and what is not yet covered, while invariants are checked after every action (C-54) |
| **Finding** | A deterministic check that failed during exploration (an invariant, or an expectation stated before the action), with the steps and build that produced it; reproduced when two replays from a fresh start both fail the same way (C-54) |
| **Testing knowledge** | The Orchestrator's record per project of what exploration has covered, the findings and their state, and how each run did, used to choose the next tests (C-54) |
| **Child objective** | An objective a lead delegated with a directive, within its parent objective's permissions and budgets; its thread and result appear in the parent's thread (C-54) |
| **Directive** | A recorded instruction from one agent to another (author, recipient, parent objective, scope, status, result), created only by a tool the Orchestrator validates, never by text alone (C-54) |
| **Thread** | An objective's entries in the Conversation: the Operator's messages, directives, results and system events, with tool activity folded under each step; kept with the objective's records (C-54) |
| **Observer mode** | The visible agents (§4.16) at a speed the Operator sets: agents' typing drawn character by character, clicks, focus and scrolling, with the agent, its goal and its decisions; `instant` turns it off (C-54) |
| **Approved baseline** | The commit the Operator last approved as the reference for Agentique's architecture (the git tag `approved-baseline`, moved only by the Operator); an objective's review compares the cumulative change since it, not only the last commit (C-55) |
| **Disposition** | The lead's recorded judgment of a reproduced finding before anything is fixed: a genuine defect, a wrong expectation of the tester, an ambiguous requirement (a question for the Operator) or an unreliable reproduction; kept in the testing knowledge so a wrong expectation is not proposed again (C-55). The standard term of defect tracking |
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

**Stage 10 (C-51)**

106. Claude Agent SDK (read 2026-10-01): overview, including that third-party products may not offer claude.ai login https://code.claude.com/docs/en/agent-sdk/overview ; TypeScript reference https://code.claude.com/docs/en/agent-sdk/typescript ; permissions and their order of evaluation https://code.claude.com/docs/en/agent-sdk/permissions ; sessions https://code.claude.com/docs/en/agent-sdk/sessions ; custom tools https://code.claude.com/docs/en/agent-sdk/custom-tools ; the package `@anthropic-ai/claude-agent-sdk` 0.3.287 on npm, its `sdk.d.ts` (`Options`: `tools`, `settingSources`, `permissionMode`, `strictMcpConfig`, `env`) and `manifest.json` (Claude Code 2.1.287 and the binaries' checksums)
107. Claude Code environment variables (read 2026-10-01): `CLAUDE_CONFIG_DIR`, `CLAUDE_CODE_DISABLE_AUTO_MEMORY`, `CLAUDE_CODE_DISABLE_CLAUDE_MDS`, `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC`, `DISABLE_TELEMETRY`, `ANTHROPIC_API_KEY` https://code.claude.com/docs/en/env-vars

**Stage 12 (C-54)**

108. Claude Code authentication and the Agent SDK's credentials (read 2026-10-04): the order in which credentials are chosen (cloud providers, `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_API_KEY`, `apiKeyHelper`, `CLAUDE_CODE_OAUTH_TOKEN`, Anthropic profiles, the `/login` subscription), where they are stored (`%USERPROFILE%\.claude\.credentials.json`, or under `CLAUDE_CONFIG_DIR`), and `claude setup-token`'s one-year token for CI pipelines and scripts, set as `CLAUDE_CODE_OAUTH_TOKEN`, which authenticates with the Operator's Claude subscription https://code.claude.com/docs/en/authentication ; the Agent SDK quickstart's note that, unless previously approved, third-party products built on the SDK may not offer claude.ai login https://code.claude.com/docs/en/agent-sdk/quickstart ; `claude auth status` (JSON; `authMethod` is one of `none`, `claude.ai`, `oauth_token`, `api_key`, `api_key_helper`, `third_party`; exit 0 when logged in) https://code.claude.com/docs/en/cli-reference ; `Query.accountInfo()` https://code.claude.com/docs/en/agent-sdk/typescript and, in `@anthropic-ai/claude-agent-sdk` 0.3.287's `sdk.d.ts`, `AccountInfo` (`subscriptionType`, `tokenSource`, `apiKeySource`, `apiProvider`) and the init message's `apiKeySource` (`'none'` for a claude.ai login)
109. Claude models and prices (read 2026-10-04): `claude-opus-5-5` $4 input, $20 output, $0.20 cache reads per million tokens, effort `low` to `max` with `medium` the default, thinking always on; `claude-sonnet-5-5` $2 / $10, cache reads $0.20, effort default `high`; `claude-haiku-4-5` $1 / $5 https://platform.claude.com/docs/en/about-claude/models/overview and https://claude.com/pricing ; DeepSeek's `GET https://api.deepseek.com/models` (read 2026-10-04: `deepseek-flash`, DeepSeek-V4.1-Flash, and `deepseek-v4-pro`, efforts `low`, `high`, `max`)
