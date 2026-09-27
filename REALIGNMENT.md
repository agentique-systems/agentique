# REALIGNMENT

**Status:** Governing direction for Agentique. This document takes precedence
over every other document, register and agent instruction in this repository
that conflicts with it, including `AGENTS.md`, `README.md`, `docs/`,
`requirements.json`, `standards/*coverage*.json`, `verification/` and the
original `Agentique-Specification-v0.1.html`. Where they disagree, this document
wins until those files are reconciled (see §7).

**Date:** 2026-09-26
**Baseline inspected:** branch `platform/native-studio-alpha-acceptance` at
`b09a9059` (61 commits ahead of `main`)
**Origin:** a discovery interview with the project owner (the Operator),
together with a bounded, read-only investigation of the repository.

**How to read the labels**

| Label | Meaning |
|---|---|
| **C-n** | Confirmed decision by the Operator |
| **R-n** | Recommendation (engineering judgment, open to revision) |
| **A-n** | Assumption not yet verified |
| **Q-n** | Open question |
| *(provisional)* | The recommendation depends on a spike or experiment |

Section 6 lists all of them in one place.

---

## 1. Vision and purpose

### 1.1 What Agentique is

Agentique is a **native desktop application in which a person and AI agents
design, simulate and implement systems together, working at the level of
system architecture rather than code.**

It is where a project's **development** lives for the project's whole
lifecycle (C-1). The project's artifacts (source code, repositories, running
services) live wherever they normally would. Agentique links to them and
checks them; it does not host them (C-1, C-7).

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
research instead of a vocabulary an AI invented. That borrowed rigor is why
they were chosen (C-4).

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
   its intent. This is a fine line: agents must follow instructions *and*
   understand what the person actually means.

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

### 1.4 Dogfooding, and this repository as evidence

This repository is itself an example of what Agentique is meant to prevent.
It was produced largely by AI agents working without a tool like Agentique,
and it shows every failure mode above (§4.1). Agentique must eventually be able
to rescue repositories like this one, and Agentique must itself be designed by
the same principles it promises (C-13, C-20).

### 1.5 What Agentique is not (C-14)

| Non-goal | Horizon |
|---|---|
| A certified or fully conformant KerML/SysML implementation; interchange with other SysML tools is not required | Not a goal. Completeness may grow where it pays off (C-4) |
| A tool where users read or write SysML text; SysML sits entirely beneath the Surface and the agents | Never |
| A multi-user, collaborative or cloud-hosted service | Not now; one Operator on one machine |
| The home of a project's source code or deployments | Never; Agentique links to them |
| An autonomous multi-agent factory | Not yet; one Assistant first, orchestration later (C-5) |
| A physics or physical-systems simulator | Not the aim; simulation tests architecture and contracts (C-16) |
| A generator of verification paperwork for its own sake | Never; proof is working software the Operator uses (C-15) |

---

## 2. Intended experience

The scenarios describe the experience Agentique must reach, in the order its
capabilities are built (C-17). Once a capability exists, it is available at
any time in a project's life. **Using Agentique is a continuous loop, not a
pipeline** (C-17): architecture changes after code exists, the new architecture
is simulated, and it is then re-implemented.

### 2.1 Scenario A: first proof, a small new system (C-18, C-19)

The example system is a **URL shortener** with an API, storage and click
statistics. Any comparably small software system will do.

| Step | What the Operator does | Observable success | Failure and recovery behaviour |
|---|---|---|---|
| A1 | Opens Agentique, creates an empty project and tells the Assistant the idea in ordinary language. | The project opens quickly on a fresh machine with no special install step beyond the app and a Claude API key. The Assistant replies in the Conversation. | A missing or invalid API key produces a clear message. The Surface still works fully by hand. |
| A2 | Watches the Assistant build the architecture. | Components, interfaces, connections and requirements **appear on the Surface as they are created**. Each Assistant action is visible in the Conversation as it happens (C-8). Names are standard and plain. | The Assistant asks when it meets a major decision (e.g. separate statistics service or not) instead of guessing (C-6). The Operator can stop it mid-work, and partial work stays visible and can be undone. |
| A3 | Explores and adjusts the architecture both visually and in conversation until satisfied. | Direct edits on the Surface (create, connect, rename, move, delete, set properties) and edits made through conversation have the same effect on the same System State. Invalid combinations are flagged at the element concerned, in plain language. | An invalid edit never silently corrupts the state: it is shown as invalid and can be fixed or undone. |
| A4 | Locks the parts they consider settled. | Locked parts are visibly marked. | The Assistant cannot change a locked part without asking (C-11). |
| A5 | *(Stage 4)* Defines and runs simulation scenarios, such as "create a link, then resolve it, then read the stats". | The scenario steps through the modelled parts and interfaces on the Surface. It reports clearly whether the architecture can carry it out and whether the requirements hold. | An impossible step (missing interface, wrong item type, unhandled message) is shown at the exact element, with no guessed outcome. A finished run is not reported as a passed check. |
| A6 | *(Stage 5)* Asks the Assistant to implement the system. | The code is written in an ordinary external git repository. Each component is linked to its code and tests. Agentique shows per component whether it is implemented and whether its checks pass. | Code that contradicts the model (e.g. a missing interface operation or a forbidden dependency) is shown as **drift** on the affected part. |
| A7 | Runs the resulting system and tries it. | The URL shortener works as designed. | — |
| A8 | Days later, brings a loosely worded new idea: "add expiring links". | The Assistant first maps the idea onto the architecture and **shows what would change** before changing it. Unaffected parts are untouched. | If a locked part must change, the Assistant asks and explains why; the Operator confirms or refuses. A refusal leaves everything as it was. |
| A9 | Closes Agentique and reopens it later. | Everything is exactly as left: architecture, locks, history, conversation and links. History is browsable, with a visual "what changed" view between points in time. | After a crash, at most the work since the last save is lost, and the state is never corrupted. |

**Stage mapping.** Steps A1–A4, A8 (at architecture level) and A9 form the
Stage 3 proof. A5 is Stage 4. A6–A7 are Stage 5, which completes Scenario A
(C-19).

### 2.2 Scenario B: a larger system (C-18)

The same journey applies to a system with many components and several
interacting subsystems. What it adds:

- The Surface stays **readable and fast** at that size through focus,
  grouping, levels of detail and search.
- The Assistant keeps concepts **general** instead of multiplying near-duplicate
  parts. This is the "shallow concepts" failure (C-10).
- Several ideas can be explored on **branches** and compared visually before
  one is kept (R-6).

### 2.3 Scenario C: Agentique builds Agentique (C-13, C-18)

Agentique's own architecture (§3.6) is opened in Agentique. The Operator
explores it, and the Assistant makes a real change to Agentique, with its code
linked and checked (Stage 5 capabilities). Its locked core must stay intact.
This is also the proof that Agentique can take on an existing, messy codebase.
That means reading an existing codebase into a model before switching to "model
as contract", which is a capability Scenario A never needs.

### 2.4 Quality bar for the app itself (C-21)

Agentique is judged as a product the Operator wants to use every day. **A very
high standard of visual design, features and user experience is expected.**

**The Surface**

- **Aesthetic.** Modern and polished, inspired by current developer tools and
  AI products. Dark-first with a coherent light theme, refined typography,
  consistent spacing, restrained colour and purposeful motion. It must not look
  like a debug tool or a default widget toolkit.
- **Feel.** Smooth, immediate panning, zooming and selection at "Unreal Engine
  for systems" quality, with no stutter during ordinary work.
- **Liveness.** When the Assistant changes something, the change is visibly
  animated or highlighted where it happens, so the Operator can follow along.
- **Clarity.** Clear containment, interfaces and connections. Levels of detail
  are tuned so that large systems stay understandable. Labels never clip into
  illegibility.
- **Direct manipulation.** Create, connect, rename, move and delete by hand.
  Layout is presentation only and never changes the model (retained from the
  original AGQ-UI02).
- **Keyboard and command palette** for everything, with a searchable command
  palette and good defaults.

**The Conversation panel** must match the quality of modern AI chat products:

- streamed responses, rendered Markdown and code blocks;
- each tool action shown as a compact live card (what it is doing, to which
  element, and the result), expandable for detail;
- element references in chat are **clickable** and select the element on the
  Surface; selecting on the Surface can be referenced in chat;
- a stop button that works immediately, retry and edit-and-resend;
- questions from the Assistant ("major decision", "locked part") are shown as
  clear, answerable prompts, not buried in prose;
- conversation history is kept per project and survives restart;
- errors (network, API limits, invalid tool calls) are explained plainly and
  never lose the Operator's input.

**Judging quality.** Quality is judged by the Operator using the app (C-15).
Screenshots and automated UI tests support that judgment; they don't replace
it. Whether the current UI toolkit (egui) can reach this bar is an early
investigation (Q-6).

---

## 3. Engineering principles and boundaries

### 3.1 Responsibilities

| Actor | Responsibility | Not its responsibility |
|---|---|---|
| **Operator** | Intent, high-level decisions, approving changes to locked parts, judging quality | Low-level implementation |
| **Assistant** (later the Orchestrator) | Turning intent into changes to the System State through typed tools; mapping new ideas onto the architecture first; asking about major decisions; later, implementation and simulation | Silently changing locked parts; acting outside its tools; claiming results it did not observe |
| **Language core** | Meaning and validity of the System State under the chosen KerML/SysML subset | UI, AI transport, persistence format |
| **System State service** | Holding the live state; applying changes atomically; enforcing locks; publishing change events | Deciding intent |
| **History (git)** | Durable checkpoints, branches and past states | Live editing state |
| **Claude API (model provider)** | Language reasoning behind the Assistant | Authority over the System State; being required for manual work |

### 3.2 Authority and autonomy (current policy)

- **One operation boundary.** The Surface and the Assistant change the System
  State through the **same typed operations**. Nothing bypasses them, no
  direct state or file mutation (retained from the original AGQ-UI01; C-2).
- **The Assistant acts where it is confident** and asks the Operator about
  major decisions (C-6). *What counts as major* is defined in the Assistant's
  skills and refined through use (Q-3).
- **Locked parts** can change only after the Operator's explicit confirmation of
  that specific change (C-11). A lock covers the part and its interfaces
  (R-11). Locks are stored in the System State and versioned with it (R-6).
- **Visibility instead of approval-for-everything.** Every Assistant action is
  visible live (C-8). This **supersedes** the original rule that every semantic
  change needs prior approval (AGQ-AI01, §6.5).
- **Always recoverable.** The Operator can stop the Assistant at any time and
  undo its changes (R-12). A failed provider never blocks manual work
  (retained from the original spec).
- **The Assistant's output is untrusted input.** Tool calls are validated like
  any other change; the model is never a semantic authority (retained).
- **Future autonomy policy.** Finer autonomy levels, per-agent permissions and
  orchestration are deferred (Q-3, Q-4).

### 3.3 The role of KerML/SysML (C-4, C-20)

- **Foundation, not scripture.** Use enough KerML/SysML that Agentique and
  everything built with it rests on a stable core, standard parts and standard
  language. Don't read the specification as gospel or chase 100% completeness.
  Completeness may grow later where it pays off.
- **A deliberate subset, grown by need.** Start with the constructs the
  scenarios require. That is roughly packages, part/port/interface/item/
  attribute definitions and usages, connections and interfaces, specialisation,
  redefinition, multiplicity, requirements, and later states/actions for
  simulation. Add a construct when a real scenario needs it.
- **Record what is excluded and why** (the Operator's concern "what would we be
  missing out on?"). The subset manifest lists supported, partially supported
  and deliberately excluded constructs, each with a reason (R-8).
- **Standard terms, standard meaning.** Where Agentique supports a concept, it
  uses the standard's name and meaning. Deviations from the standard are
  allowed when the standard is wrong, ambiguous or impractical. Each deviation
  is recorded in one short list with its reason, with no versioned profile
  apparatus (R-9).
- **Unsupported means explicit.** An unsupported construct is reported as
  unsupported, never silently approximated (retained from AGQ-STD02).
- **Invisible to users.** SysML text is a storage and interchange format for
  Agentique and its agents, not a user interface (C-4). The pinned OMG
  specifications and library bytes remain the reference (§4.3).

Three separate claims are kept apart:

| Claim | Status |
|---|---|
| **Standards conformance** (Agentique implements KerML/SysML correctly and completely) | Not a goal |
| **Model validity** (the System State is internally consistent under the supported subset: types exist, connections fit, parts compose) | Required. This is what makes the Lego pieces fit |
| **Goal satisfaction** (the built system does what was intended) | The real target, shown by simulation, implementation checks and the Operator's own use |

### 3.4 Relationship between model and code (C-7)

- **Working direction: "the model is the contract".** Implementation lives in
  external repositories. Model elements are linked to code, tests and (later)
  running services. Agentique runs checks that the code honours the model and
  reports differences as drift.
- **Long-term goal: two-way reconciliation.** Either side may change, and
  Agentique proposes how to reconcile them.
- Both are **provisional** and to be settled by Stage 5 experiments. Which
  checks are feasible (interface signatures, dependency direction, required
  tests, test results) is decided there (Q-2).
- **Linking to deployed systems** (e.g. services in a Kubernetes cluster) is a
  later, separate kind of link: observation, not authoring (C-7).

### 3.5 History and persistence (C-9 delegated; R-6, provisional)

The Operator delegated this decision. The recommendation:

1. The System State is saved as **SysML text files** plus a small **identity
   and lock file**, in a **git** repository.
2. By default these live in a folder inside the project's own code repository,
   so that model changes and the code that implements them can share commits
   (Stage 5). A standalone model repository is also allowed.
3. The **live state lives in the running app** and is saved to disk
   continuously. Agentique creates commits at meaningful points (a finished
   Assistant task, or an explicit Operator checkpoint), with messages
   describing the intent.
4. Short-range undo is Agentique's job. Longer-range history means git commits.
   Branches are for exploring ideas.
5. Agentique adds what git lacks: stable element identities across text edits,
   a **visual architectural diff**, and later semantic merge.
6. **Spike before committing to this** (Stage 1). It must show three things:
   reloading from files is fast enough; identities survive rename and move;
   branch conflicts on the same element are handled sensibly.

This replaces the custom SQLite revision and branch stores
(`adapters/modeling-sqlite`, `adapters/storage`), which are themselves a
home-grown version-control system.

### 3.6 Agentique designed by its own principles (C-20)

Everything Agentique promises other systems applies to Agentique first.

- **Agentique has a maintained SysML architecture model** in `models/agentique/`.
  It is rewritten to describe the *realigned* architecture (Stage 0) and kept
  current. Until Agentique can open itself (Scenario C), agents maintain it as
  text.
- **The model is the contract, applied to our own code.** Each major part in
  the model maps to a crate or module. The model's connections define the
  allowed dependencies between them. A simple automated check fails if the
  crate dependency graph contradicts the model (R-15). This is the "model as
  contract" principle (C-7) applied to ourselves before Agentique can do it for
  others.
- **Locked core.** Parts of Agentique marked as stable core in its model (the
  language core, System State operations and the persistence format) change
  only with an explicit, recorded Operator decision (C-11, R-16).
- **Standard names.** Model concepts use KerML/SysML names; everything else
  uses plain, widely understood software terms (§7.4).
- **Generalise before multiplying.** Two parts that do nearly the same thing
  are one concept with parameters. A new crate or top-level module must be
  justified by a distinct responsibility in the self-model.

**Provisional logical architecture (R-14).** This describes responsibilities,
not a crate layout. It will be expressed in `models/agentique/` and refined.

```text
Agentique
├── LanguageCore          KerML/SysML subset: element graph, text parse/print,
│                         name resolution, typing, specialisation, validation
├── SystemState           live state of one project; typed change operations;
│                         locks; undo; change events; queries for views
├── History               git-backed save/load, checkpoints, branches, diffs
├── Studio                application shell
│   ├── Surface           spatial rendering and direct manipulation
│   ├── Panels            inspector, requirements, history, simulation, links
│   └── Conversation      chat with the Assistant
├── Assistant             Claude API client, tool-use loop, skills, tools that
│                         call SystemState operations (later: Orchestrator)
├── Simulation            (Stage 4) scenarios over the architecture; runs,
│                         traces and separate check verdicts
└── ImplementationLinks   (Stage 5) links from model elements to code, tests
                          and (later) running services; drift checks

Allowed dependencies (arrows point inward):
  Studio → SystemState, Assistant → SystemState
  SystemState → LanguageCore, SystemState → History
  Simulation → LanguageCore, ImplementationLinks → SystemState
  LanguageCore depends on nothing above it. No UI, network or AI types in
  LanguageCore or SystemState (retained AGQ-EXT01).
```

### 3.7 Quality expectations for the code

- **Simple before clever.** Pick the least complex design that meets the
  current stage's scenario. Build for the next stage only when that stage
  starts.
- **Stable core, experimental edges.** Experiments go in edge modules, behind
  flags or on branches, never in `LanguageCore` or `SystemState` operations.
- **One of each thing.** One language engine, one application, one persistence
  mechanism, one assistant layer.
- **Readable by the Operator.** An engineer, or the Operator with an agent's
  help, can understand any part from its model element, its README and a short
  read of the code.
- **Tests prove behaviour, not ceremony.** Automated tests cover the language
  core, state operations, persistence and tool contracts. Test the journeys the
  Operator uses, not the evidence apparatus.
- **Performance is part of correctness.** An Assistant edit that takes minutes
  to appear breaks the core promise. Measure the Scenario A model regularly.
  Acceptance means edits and reloads *feel live* to the Operator (§5, Stage 1).

---

## 4. Current-state alignment

### 4.1 What exists (evidence map)

| Area | What it is | Evidence | Actual state |
|---|---|---|---|
| Original specification | v0.1 build spec: modelling, bounded simulation, Console, approved-change Assistant, self-model | `Agentique-Specification-v0.1.html`, `requirements.json` | Clear, modest, largely sound. Superseded in places (§6.5) |
| **Generation 1** | Model, parser, semantics, workspace, simulation (AGQ-SEQ-01), application (approvals, receipts), HTTP server, CLI, SQLite storage, OpenAI-style assistant adapter, React console | `crates/{model,syntax,semantics,workspace,simulation,application,server,cli}`, `adapters/{storage,assistant}`, `console/`, `verification/RELEASE.md` | "Release candidate, not completed". The demo loop works per its records. The assistant was never used with a real provider. `npm run verify` is intentionally red |
| **Generation 2 language engine** | Kernel element graph, generated KerML/SysML descriptors, lossless parser, lowering, semantics with producer closure, publication, certificates and audits | `crates/{kernel,kerml,kerml-syntax,kerml-text,kerml-semantics,sysml,sysml-semantics,standard-libraries}`, `tools/metamodel-gen`, `standards/v2-coverage.json` | Rigorous and extensive. About 60–65k hand-written non-test lines, about 70k generated, about 60k test. Roughly **50–65% of hand-written semantics/text code is conformance and publication machinery** (closure, receipts, profiles v1–v9, audits). Estimated, not audited line by line |
| Runtime bundle | 610 MB authenticated `.agq-runtime` holding precomputed library "publications" | `crates/runtime-publications`, `docs/runtime-publication*.md`, `docs/runtime-rematerialization.md` | Needed only because of the certification design. Cache bytes were lost once and had to be rebuilt by replaying history |
| Modelling platform | In-memory workspace, durable repository, modelling service, views, agent types | `crates/modeling-{workspace,repository,service,view,agent}`, `adapters/modeling-{sqlite,api,http}` | Works over Gen2. **Every edit is a full cold rebuild**: about 128 s to prepare a change and 6.5 GB peak memory on the 6-document real model. About 175 s to the first validated view |
| **Native Studio** | egui 0.33 + wgpu 27 desktop app with a custom GPU scene renderer | `crates/{studio-native,studio-scene,studio-platform}`, `docs/native-studio*.md`, `docs/adr/0030-*` | Real, in-process boundary, promising renderer. **No conversation panel, no real LLM** (hard-coded mock intent). Only create-nested-part and rename edits. Own judgment: "NOT YET ACCEPTED" (`verification/native-studio-alpha/FINAL.md`). About 3.9k-line UI automation harness compiled into the production binary |
| Browser Studio prototype | axum host + React client over the same platform | `crates/studio`, `console/src/studio/` | Redundant with Native Studio |
| Assistant / agents | Gen1: one OpenAI-style call, one tool, no loop. Gen2: authority and proposal types with a mock decision | `adapters/assistant/src/lib.rs`, `crates/modeling-agent/src/lib.rs` | **No Claude integration anywhere; no tool-use loop anywhere** |
| Simulation | AGQ-SEQ-01: one flat state machine, deterministic trace, run separate from verdict | `crates/simulation/src/lib.rs` (about 1.1k lines) | Tied to Gen1. Useful as concepts, not reusable as code |
| Self-model | Original 4 files, plus a newer 6-file model of the Gen2 platform | `models/*.sysml`, `models/agentique/` | Describes the old architecture |
| Verification apparatus | Receipts, logs, screenshots, generated obligation dumps | `verification/` (about 4,983 files, about 3.6 GB, about 21M lines; one file is 5.2M lines), `tools/verify.mjs`, `tools/report.mjs`, about 43 `verification/scripts/*.py` | Mostly machine output. Did not produce Operator confidence |
| Decision records | 30 ADRs, mostly KerML errata/"operational profile" rulings; 9 "authority conflict" documents | `docs/adr/`, `docs/kerml-*-authority-conflict.md` | Historical |
| Repository hygiene | 295 local branches, 65 worktrees, one prerelease runtime asset, two Cargo workspaces (Studio excluded from the root) | `git branch`, `git worktree list`, root `Cargo.toml` | Hard to navigate |
| CI | `ci.yml` (intentionally red `checks` job), `native-acceptance-build.yml`, `runtime-asset.yml` | `.github/workflows/` | Red by design, so it no longer signals anything |

**Diagnosis.** The original, modest specification drifted twice:

1. **Toward standards conformance as a goal in itself.** This produced about
   200k lines of language engine, nine profile versions and certificates.
2. **Toward a showcase UI on top of that engine.**

Both drifts happened under heavy rules and verification. The result shows every
slop failure mode:

- **Unnecessary complexity:** certification machinery for a single-user
  desktop app.
- **Shallow, duplicated concepts:** two engines, three user interfaces, two
  stores, two assistant layers.
- **Invented names:** "Operational v9", "producer frontier", "Agent Fabric",
  "System World", "rematerialization".
- **Experimentation at the core:** the core was revised nine times while the
  product loop never closed.

This is evidence that **more process does not prevent slop**. Clear goals, a
visible architecture and protected decisions are what prevent it.

### 4.2 Direction for each area

Dispositions:

- **Retain:** keep as is, with minor changes.
- **Simplify:** keep the useful core and remove the rest.
- **Consolidate:** merge into one.
- **Retire:** preserve at the archive tag (§5, Stage 0), then remove from the
  active tree; git history is never deleted (C-22).
- **Investigate:** a spike decides.

| Area | Disposition | Rationale | Trace |
|---|---|---|---|
| `crates/kernel` | **Retain, simplify** *(provisional)* | Generic element graph, the right foundation. Remove proof/search-dependency and publication types that the working path does not need | C-4, C-20; R-3 |
| `crates/kerml`, `crates/sysml` (generated descriptors), `tools/metamodel-gen` | **Retain** | Standard vocabulary generated from the pinned metamodel, cheap to keep, anti-invention | C-4 |
| `crates/kerml-syntax` | **Retain** | Lossless, error-tolerant parser of the storage format (SysML text in git) | R-6 |
| `crates/kerml-text` | **Simplify** *(provisional)* | Keep lowering, source projects and identity reconciliation. Remove `sysml/publication*`, authenticated input and runtime restore | C-4, C-14 |
| `crates/kerml-semantics`, `crates/sysml-semantics` | **Simplify, or rebuild the core simply** *(provisional, Stage 1 spike)* | Keep resolution, namespaces, typing, inheritance, implicit specialisation, featuring, validation and SysML usage typing. Remove producer closure, publication, certificates, audits and profile versioning. Separability is uncertain, because evidence types run through the generic queries | C-4, C-14; R-3 |
| `crates/standard-libraries` | **Retain, simplify** | Load pinned library sources. Consider loading only the libraries the subset needs, computed at startup or cached locally without authentication | R-4 |
| `crates/runtime-publications`, the `.agq-runtime` bundle, `tools/runtime-recovery`, `tools/restore-accepted-kerml-transport.py`, `tools/verify-runtime-distribution.py`, `tools/sysml-publication-stale.mjs`, `tools/sysml-lock-compatibility.mjs`, `.github/workflows/runtime-asset.yml` | **Retire** (when Stage 1 removes the dependency) | Exists only for certification. Causes the 610 MB install and multi-minute open | C-4, C-14 |
| `crates/modeling-workspace`, `modeling-service` | **Consolidate** into `SystemState` | Keep typed change operations, stale-base rejection and atomic apply. Replace the full cold rebuild with incremental updates | C-2; R-14 |
| `crates/modeling-repository`, `adapters/modeling-sqlite` | **Retire** (when Stage 2 git persistence lands) | Replaced by git history | R-6 |
| `crates/modeling-view` | **Retain, simplify** | Projections for the Surface | C-3 |
| `crates/modeling-agent` | **Simplify** into the `Assistant` / lock policy | The authority idea is useful; "Agent Fabric" naming and the mock go | C-5, C-11 |
| `crates/studio-native`, `studio-scene`, `studio-platform` | **Retain, consolidate** | The product shell and renderer. Merge into the root Cargo workspace (the separate lockfile only protected the certified dependency set). Move the automation/soak/stress harnesses out of the production binary. Rename "Worlds" to plain view names | C-3, C-21 |
| egui as UI toolkit | **Investigate** (Stage 2) | Must reach the visual bar in §2.4, especially the chat panel. The earlier bake-off tied egui with Slint on different criteria | C-21; Q-6 |
| Generation 1: `crates/{model,syntax,semantics,workspace,simulation,application,server,cli}`, `adapters/{storage,assistant}` | **Retire** | Superseded engine and a multi-actor HTTP design not needed for one Operator. Carry forward the *concepts* in §4.4 | C-9, C-14 |
| `console/`, `crates/studio`, `adapters/modeling-http`, `adapters/modeling-api`, `tests/browser`, `tests/studio`, `playwright*.config.ts` | **Retire** | Browser and HTTP paths; the product is native only | C-9 |
| Root npm toolchain (`package.json`, `node_modules`) | **Simplify** | Keep only what the retained pinning and generation tools need. Rewrite them in Rust or Python if that removes Node entirely *(investigate)* | R-17 |
| `tools/`: `pin-kerml.mjs`, `pin-sysml.mjs`, `normative-artifacts.mjs`, `sysml-artifacts.mjs`, `library-release.mjs`, `kerml-grammar/`, `sysml-grammar/`, `metamodel-gen/` | **Retain** | Needed to regenerate descriptors and grammar from pinned sources | C-4 |
| Other `tools/` (`verify.mjs`, `report.mjs`, `registers.mjs`, `extract.mjs`, `standards-check.mjs`, `baseline.mjs`, `gen2-api-inventory.mjs`, `process-test.mjs`, pilot tools and `ValidatePilot.java`, `native-bakeoff/`, `native-acceptance-build.ps1`, `benchmark.ps1`, `__pycache__`) | **Retire** | Gen1 release registers, conformance pilots, finished experiments | C-15 |
| `verification/` (entire) | **Retire** | Evidence of a superseded direction. A new, small `verification/` is not recreated; see §7.3 | C-15 |
| `standards/artifacts/`, `standards/libraries-*`, `standards/baseline-lock.json`, `standards.references.json`, `KerML.pdf`, `SysML.pdf`, `SysAPI.pdf` | **Retain** | Pinned reference specifications and original library bytes | C-4 |
| `standards/coverage.json`, `v2-coverage.json`, `gen2-api-*.json`, `api-coverage.json`, `kerml-1.0-constraint-authority-map.json`, other conformance registers | **Retire**; replaced by one subset manifest (R-8) | Conformance-tracking registers | C-4 |
| `docs/` (all files) and `docs/adr/` (30 ADRs) | **Retire**, and carry forward still-valid decisions into §6 of this document | Superseded or historical. One current document per topic afterwards (§7.2) | §7 |
| `models/*.sysml` (original 4) and `models/agentique/` | **Rewrite** as the realigned self-model (§3.6) | The self-model must describe the real architecture | C-20 |
| `requirements.json`, `contracts/engine-operations.json`, `scenarios/*.json`, `standards/coverage.json`, `verification/traceability.json` | **Retire** | Replaced by this document and the rewritten self-model (§6.5) | §7 |
| `Agentique-Specification-v0.1.html` | **Retain** as a historical source document | Preserved original intent. Superseded parts are listed in §6.5 | AGENTS.md preservation rule |
| `AGENTS.md`, `README.md` | **Rewrite** (Stage 0) | They currently enforce the old direction | §7 |
| CI workflows | **Replace** with one green-by-default workflow (fmt, clippy, tests, self-model dependency check) | A permanently red CI carries no signal | §7.3 |
| Branches (295), worktrees (65), prerelease runtime asset, untracked `verification/native-studio-acceptance/` files, `target-phase2-frontend/`, `test-results/` | **Clean up** | After the archive tag: prune merged or obsolete branches and worktrees, mark the prerelease obsolete, delete local build leftovers | C-22 |

### 4.3 Preservation rules that remain in force

- The pinned OMG specifications (PDFs, HTML), library archives and original
  library bytes are preserved unchanged, with their hashes.
- Acquiring standards artifacts is never a build step.
- The original specification HTML is preserved as history.
- Retired work is recoverable from the archive tag (C-22).

### 4.4 Concepts to carry forward (not code)

- **Stable element identity.** Identity is separate from names and paths;
  rename and move preserve it. Ambiguous text edits need reconciliation, not
  silent reassignment (AGQ-MOD01; `crates/kerml-text/src/project.rs`
  `SourceProject` is the better base).
- **Atomic change against a known base state.** An invalid change never
  replaces the valid state; a stale change is rejected (AGQ-MOD02;
  `crates/workspace`, `crates/modeling-service`).
- **Unsupported semantics fail loudly.** They are never approximated (AGQ-STD02).
- **Simulation disciplines, for Stage 4.** Deterministic, normalised traces. A
  *run completing* is separate from a *check passing*. Runs are pinned to the
  model state they started from and isolated from each other and from the
  design. Explicit stop reasons (`condition_met`, `input_exhausted`,
  `ambiguous_transition`, limits). No external side effects during simulation
  (AGQ-SIM01–06, AGQ-SEC01; `crates/simulation`).
- **Durability practices.** Write atomically, never acknowledge before it is
  durable, refuse an unknown format version (`adapters/storage`), now applied
  to git-backed files.
- **Provider isolation.** AI transport never enters the language core, and
  provider failure never blocks manual work (AGQ-EXT01, AGQ-AI01).
- **Renderer work.** The instanced GPU scene, level-of-detail labels and hit
  testing in `crates/studio-native/src/gpu.rs` and `crates/studio-scene`.

---

## 5. Realignment sequence

Each stage ends with something the Operator **uses** (C-15). Automated tests
support each stage but are never the acceptance on their own.

**Rule for all stages: no work outside the current stage.** Items listed under
"waits" do not start early.

### Stage 0: Archive and clean (prerequisite; requires separate authorization)

**Outcome.** A small, navigable repository whose documentation tells one story.

1. Decide what to do with the untracked
   `verification/native-studio-acceptance/…` files and the 61 unmerged commits
   on `platform/native-studio-alpha-acceptance`. Recommendation: merge the
   branch to `main` as is, then tag.
2. Create the annotated tag **`archive/pre-realignment`** on `main`.
3. Remove everything marked **Retire** in §4.2 that nothing on the retained
   path depends on:
   - Generation 1;
   - the browser, HTTP and console paths;
   - `verification/`, the conformance registers and `docs/`;
   - the retired `tools/`;
   - the old CI.

   Items still depended on (runtime bundle, SQLite repository) are retired in
   Stages 1–2.
4. Merge `crates/studio-native` into the root Cargo workspace. Remove automation
   harnesses from the production binary; keep them as test code only if they
   test journeys.
5. Rewrite `AGENTS.md` (§7.1), `README.md` (what Agentique is, how to run it,
   current stage) and `models/agentique/` (the §3.6 architecture as SysML).
6. One CI workflow that is green on a healthy tree.
7. Prune branches and worktrees, and mark the prerelease runtime asset
   obsolete.

**Acceptance evidence**

- The root workspace builds.
- Tests for retained crates pass.
- CI is green.
- `README` and `AGENTS.md` match this document.
- The Operator can list the remaining top-level folders and crates and say what
  each is for.
- The archive tag checks out and builds the old state (spot check).

**Depends on:** Operator authorization.

**Stops now:** all conformance, publication, runtime-bundle and
verification-record work; Gen1 release obligations; alpha-acceptance gates.

### Stage 1: A fast, honest language core (spikes plus consolidation)

**Outcome.** The architecture of the Scenario A system can be written as SysML
text, loaded, validated and edited **interactively**, with no runtime bundle.

- **Spike 1: language core.** Model the URL shortener in the subset. Measure
  load, validate and single-edit times on the retained Gen2 core, with
  closure, publication and audit bypassed. Decide **extract** (simplify Gen2)
  or **rebuild** (write the subset core more simply, reusing the parser,
  descriptors and kernel). The deciding criteria are: edits feel live; the code
  is understandable; unsupported constructs are explicit.
- **Spike 2: git persistence.** Save, load, identity and branch-conflict
  behaviour as in §3.5.
- Produce the **subset manifest** (R-8) and the **deviations list** (R-9).
- Retire `crates/runtime-publications` and the certification modules once the
  core no longer needs them.

**Acceptance evidence**

- The Operator watches a demo, or runs a small command, that loads and
  validates the URL shortener model and reports a deliberate error at the right
  element.
- Measured times are recorded in the stage summary.
- Tests of the core pass.
- The decision (extract or rebuild) is recorded in §6.

**Depends on:** Stage 0.

**Waits:** Studio features, Assistant work.

### Stage 2: Studio foundation: the Operator edits the System State

**Outcome.** The Operator builds and changes an architecture **by hand** in
the Studio, with persistence, history and locks.

- `SystemState` with typed operations (create, delete, rename, move, connect,
  set property, lock/unlock), change events, undo/redo.
- The Surface reflects every change live. Inspector, requirements and history
  panels. A visual diff between two commits.
- Git-backed save, load, checkpoints and branches (R-6). SQLite repository
  retired.
- Visual quality pass toward §2.4, and the toolkit investigation (Q-6)
  resolved.

**Acceptance evidence.** The Operator performs Scenario A steps A3, A4 and A9
**by hand** on a fresh project. Closing and reopening restores everything, and
a forced kill loses at most the unsaved edit. The Operator judges the look and
feel acceptable for continued daily use.

**Depends on:** Stage 1.

**Waits:** Assistant.

### Stage 3: The Assistant (Claude API): first proof, architecture level

**Outcome.** Scenario A steps **A1–A4, A8 and A9** work with a real Assistant.

- Claude API client, streaming, and a **tool-use loop**. Tools call
  `SystemState` operations only.
- Skills: KerML/SysML modelling, Agentique conventions, slop rules (§1.3),
  "map ideas onto the architecture first", "ask on major decisions".
- Conversation panel at the §2.4 bar: live tool-action cards, clickable element
  references, stop, question prompts, per-project history.
- Lock enforcement with a confirmation prompt, and undo of Assistant work.

**Acceptance evidence**

- **The Operator runs Scenario A (architecture part) themselves** and is
  convinced (C-15, C-19).
- Automated tests cover tool contracts and lock refusal, using a
  deterministic stand-in for the model.

**Depends on:** Stage 2.

**Waits:** Orchestrator, multiple agents.

### Stage 4: Simulation: testing the architecture

**Outcome.** Scenario step A5. Scenarios built from Agentique's own parts run
over the architecture and report whether it can carry them out and whether
requirements hold (C-16). Simulation is available at any time, before or after
code (C-17).

- Define what "simulate a contract" means for message and interface flows,
  including the minimal state/action semantics needed (Q-5). This work is
  expected to need a subset extension, recorded in the manifest.
- Carry forward the simulation disciplines in §4.4.

**Acceptance evidence.** The Operator simulates the URL shortener's main
scenarios, sees a deliberately broken interface caught before any code exists,
and fixes it.

**Depends on:** Stage 3.

### Stage 5: Implementation linking: completes the first proof

**Outcome.** Scenario steps A6–A7. The Assistant implements the system in an
external git repository. Elements are linked to code and tests, drift is shown,
and the model is the contract (C-7).

- Settle which checks give real protection (Q-2).
- Model and code share commits by default (R-6).

**Acceptance evidence.** **The Operator runs the complete Scenario A**,
including trying the generated system and the "expiring links" change, with a
locked part refusing silent change.

**Depends on:** Stage 4.

### Later stages (order to be confirmed when reached)

- **Stage 6. Scenario B (larger system):** scale, grouping, branches, and
  keeping concepts general.
- **Stage 7. Scenario C (dogfood):** read an existing codebase into a model;
  Agentique opens and changes itself.
- **Afterwards:** the Orchestrator and assistant roles (C-5), two-way
  reconciliation (C-7), deployed-system links, a finer autonomy policy.

---

## 6. Decisions and open questions

### 6.1 Confirmed Operator decisions

| ID | Decision |
|---|---|
| C-1 | Agentique is where a project's *development* lives for its whole lifecycle; the project itself (code, deployment) can live anywhere. The primary focus is software systems |
| C-2 | One System State is the single truth. The Operator changes it on the Surface and the Assistant through tools. All changes appear live |
| C-3 | The Studio has a visual Surface (with side panels) and a Conversation as equal ways to work |
| C-4 | KerML/SysML are the foundation, not scripture. Use a deliberate subset, grown by need; completeness only where it pays off. Interchange with other tools is not required. Users never see SysML text. Chosen for their accumulated research, to avoid an AI-invented home-grown model |
| C-5 | One Assistant with tools and skills first; it becomes an Orchestrator of assistants later |
| C-6 | The Assistant acts where it is confident and asks the Operator about major decisions |
| C-7 | Model to code: "the model is the contract; code linked and checked" is the working direction, two-way reconciliation is the long-term goal. Both are provisional pending experiments. Linking to deployed systems comes later |
| C-8 | All Assistant actions are visible in real time |
| C-9 | Single user (the Operator). A native Rust application on Windows. The Claude API first. The Operator delegated the history/version-control decision (see R-6) |
| C-10 | The definition of slop and non-slop in §1.3, including "stable core, experimental edges" |
| C-11 | Parts can be locked. Changing a locked part requires the Operator's confirmation. Protecting established parts against idea-driven drift is essential |
| C-12 | Architecture-first thinking and first-class visual architecture ("like Unreal Engine") are the main cure for slop |
| C-13 | Agentique will be dogfooded to fix this repository |
| C-14 | The non-goals in §1.5 |
| C-15 | Evidence means the Operator using Agentique and systems built with it, not verification records |
| C-16 | Simulation is testing of the architecture/contract, like tests for code, built from Agentique's own parts |
| C-17 | Build order: architecture, then simulation, then implementation. Use is a continuous loop, not a pipeline |
| C-18 | Proof order: a small new system, then a larger system, then Agentique itself |
| C-19 | Scenario A (URL shortener journey) is the first proof. Stage split: architecture first, implementation second |
| C-20 | Agentique's own architecture follows KerML/SysML principles, so it enjoys the same benefits it promises |
| C-21 | Very high quality in visuals, features and UX. A modern developer- and AI-inspired aesthetic. A chat panel on par with modern AI chat interfaces |
| C-22 | "Retire" means preserved at a git tag, then removed from the active repository |

### 6.2 Recommendations

| ID | Recommendation | Basis |
|---|---|---|
| R-1 | Consolidate onto one line: Native Studio shell plus a simplified language core; retire Gen1 and the browser/HTTP paths | §4.1 duplication; C-9, C-10 |
| R-2 | Remove the certification machinery (publications, profiles, receipts, runtime bundle) from the working path | C-4, C-14; the per-edit rebuild of about 2 minutes breaks C-2/C-8 |
| R-3 | Decide "extract Gen2 core" or "rebuild core simply" by a Stage 1 spike. **Decided overnight 2026-09-27, pending Operator confirmation: rebuild.** The language core is `crates/language` (`agq-language`); the Generation 2 crates are retired in Stage 2 once the Studio no longer uses them, including the generated descriptors, the parser and their generators, which the new core does not use | Separability is uncertain. Stage 1 measured Generation 2 at about 72 s per edit and 5.5 min to the first validated view without the bundle; the rebuilt core validates the URL shortener in 0.2 ms (`docs/stages.md`) |
| R-4 | Load only the standard libraries the subset needs; compute them at startup or cache them locally, with no authentication. Stage 1: only `ScalarValues` is needed, built in (deviation 1) | C-4 |
| R-6 | Git history, with SysML text plus an identity/lock file, next to the code by default; the live state is in the app; commits at meaningful points; branches for ideas; a visual diff. **Spike done; decided overnight 2026-09-27, pending Operator confirmation: keep, with git embedded through `git2` (no separate git install), a `model/` folder of `*.sysml` files plus `agentique.json` (identities and locks), continuous saving with commits at checkpoints, and merges done by element identity, never by git's text merge** | Delegated under C-9; §3.5. The spike showed git's text merge can merge cleanly yet leave a reference pointing at nothing |
| R-7 | Merge `studio-native` into the root workspace; keep test harnesses out of the production binary | §4.1 |
| R-8 | One subset manifest: supported / partial / excluded constructs with reasons | C-4 (the "what are we missing" concern) |
| R-9 | One deviations list for places Agentique departs from the standard, with reasons; no profile versions | C-4, C-10 |
| R-10 | Retire all conformance registers and verification records; replace them with stage summaries (§7.3) | C-15 |
| R-11 | A lock covers the part and its interfaces (definitions and features it owns) | C-11 |
| R-12 | Stop and undo of Assistant work are required in Stage 3 | C-6, C-8 |
| R-13 | Plain names instead of coined ones (§7.4) | C-10 |
| R-14 | The provisional logical architecture in §3.6 | C-20 |
| R-15 | Automated check: the crate dependency graph must agree with the self-model's allowed dependencies | C-7, C-20 |
| R-16 | The stable core of Agentique (LanguageCore, SystemState operations, persistence format) is locked: changes need a recorded Operator decision | C-10, C-11 |
| R-17 | Minimise the Node toolchain to what the retained pinning tools need *(investigate)* | C-10 |

### 6.3 Assumptions

| ID | Assumption | How it gets tested |
|---|---|---|
| A-1 | The retained Gen2 core, without closure and audits, can validate a small model fast enough to feel live | Stage 1 spike |
| A-2 | A small KerML/SysML subset is enough for Scenario A | Stage 1 modelling of the URL shortener |
| A-3 | SysML text in git with an identity file preserves element identity well enough | Stage 1 spike |
| A-4 | The Claude API with tool use can produce coherent architecture changes through typed tools | Stage 3 |
| A-5 | egui (with custom rendering) can reach the §2.4 bar, including the chat panel | Stage 2 investigation (Q-6) |
| A-6 | Nothing in Generation 1 is used by anyone else, so retiring it has no external cost | Operator confirmation at Stage 0 |

### 6.4 Open questions

| ID | Question | Blocking? |
|---|---|---|
| Q-1 | What "Agentic" reference the Operator mentioned as inspiration (skipped in the interview) | No |
| Q-2 | Which model-to-code checks are feasible and worthwhile | Blocks Stage 5 design |
| Q-3 | What exactly counts as a "major decision" for the Assistant; finer autonomy levels | Refined during Stage 3 |
| Q-4 | Orchestrator design: which assistant roles, how they coordinate | No (after Stage 5) |
| Q-5 | What simulation semantics are needed to test contracts (message flows, states, actions, time?) | Blocks Stage 4 design |
| Q-6 | Whether egui is the right toolkit for the quality bar, or another option is needed | Resolve in Stage 2 |
| Q-7 | How to read an existing codebase into a model (needed for dogfooding) | No (Stage 7) |
| Q-8 | Which parts of the SysML standard the Operator would miss under the subset; review the manifest together | No (reviewed as it grows) |
| Q-9 | Where conversation history is stored (in the project folder with the model, or app-local) | Resolve in Stage 3 |

### 6.5 How the original requirements are reconciled

The 20 original requirements (`requirements.json`, specification §10) are
superseded by this document. Their still-valid content lives on as follows:

| Original | Disposition |
|---|---|
| AGQ-STD01 (pinned standards for every construct) | **Superseded** by C-4 and R-8/R-9: subset by need, pinned references kept |
| AGQ-STD02 (separate support status; reject unsupported semantics) | **Retained** as §3.3 "unsupported means explicit", with one subset manifest instead of five-axis coverage |
| AGQ-SELF01 (maintained self-model linked to implementation and tests) | **Retained, revised** as §3.6 and R-15 |
| AGQ-MOD01 (stable identity through rename/move/persistence) | **Retained** (§4.4, Stage 1–2) |
| AGQ-MOD02 (atomic change sets on a base revision; invalid drafts kept separately) | **Retained** (§4.4, Stage 2) |
| AGQ-MOD03 (standard text and .kpar round-trip) | **Superseded.** Interchange is a non-goal (C-4). SysML text is the storage format; .kpar is dropped |
| AGQ-SIM01–06 (prepare, isolation, controls, reproducibility, run vs verdict, trace) | **Deferred to Stage 4.** The concepts are retained (§4.4), the AGQ-SEQ-01 specifics are not |
| AGQ-UI01 (one revision and context across Conversation and Surface; same operation boundary) | **Retained** (§3.2) |
| AGQ-UI02 (Views, layout independent of semantics, keyboard and non-diagram alternatives) | **Retained, extended** by §2.4 |
| AGQ-AI01 (typed scoped tools; approval before every commit/execution) | **Partly superseded.** Typed tools are retained; approval-for-everything is replaced by visibility, locks and questions on major decisions (C-6, C-8, C-11) |
| AGQ-SEC01 (no external effects in simulation; untrusted inputs) | **Retained** for Stage 4; untrusted Assistant output applies now (§3.2) |
| AGQ-NFR01 (durable state; no false completion after crash) | **Retained, revised** for git-backed persistence (Scenario A9) |
| AGQ-NFR02 (limits, progress/cancellation, responsiveness) | **Retained**: "feels live" (§3.7); stop and cancel for Assistant work (R-12) |
| AGQ-EXT01 (core independent of HTTP, storage, UI and AI) | **Retained** (§3.6 dependency rules) |
| AGQ-TRACE01 (requirements linked to implementation and evidence; a link is not satisfaction) | **Retained, lightened**: the self-model and R-15 give the links; satisfaction is shown in stage summaries and by the Operator's use |

The generation-1 release obligations (`standards/coverage.json`,
`verification/traceability.json`) and the `AGENTS.md` rules that preserve them
are **withdrawn**. Generation 1 will not be released.

### 6.6 Decisions carried forward from the retired records (Stage 0)

The retired `docs/` and `docs/adr/` (at `archive/pre-realignment`) held these
still-valid decisions. They apply from Stage 1 on.

| ID | Decision | Origin |
|---|---|---|
| D-1 | Inheritance is a lookup over the element graph; inherited features are never copied into the specialising type. A reference (typing, specialisation, redefinition, connection end, satisfy) holds its target's identity once resolved, so renaming or moving the target never re-binds it by name | ADR-0001, 0005, 0026; Stage 1 review |
| D-2 | Authored and implied facts stay separate. Implied relationships (implicit specialisation, implied redefinition) are derived, never written into the SysML text and never treated as authored | ADR-0001, 0026 |
| D-3 | Identity: retired identities are never reused; deleting and recreating gives a new element; identity is never re-matched by name. Library elements get identities derived from the pinned library bytes, so the identity file covers authored elements only | ADR-0006, 0012 |
| D-4 | An unresolved or wrongly typed reference never becomes a relationship. It stays a reported error at its source location; no placeholder targets | ADR-0006 |
| D-5 | Ambiguity is an error. It is never settled by identity, hash or traversal order, or "first import wins" | retired profile docs |
| D-6 | Caches are disposable. The durable truth is the SysML text plus the identity file; any cache is rebuilt from them | ADR-0028 |
| D-7 | Model and presentation are separate. Layout, camera, selection and views are presentation; removing an element from a view never deletes it; presentation undo never touches model history; diffs compare element identities | ADR-0029, 0030 |
| D-8 | The standard libraries are the pinned 2026-04 corrective release, not the original 2.0 downloads | standards-discrepancies |

Candidate deviations from KerML/SysML found in the retired errata rulings are
input to the Stage 1 deviations list (R-9), not decisions by themselves.

### 6.7 Decision log

| Date | Change | Why |
|---|---|---|
| 2026-09-27 | R-14 refined: Studio may depend on Assistant (the Conversation lives in the Studio and drives the Assistant). Recorded in `models/agentique/` | The Stage 0 dependency check (R-15) found the Studio already uses the Assistant crate; the dependency is inherent, not accidental |
| 2026-09-27 | R-15 implemented with the standard SysML `dependency` relationship between part definitions (§3.6 says "connections"). Every allowed dependency is listed; they are not transitive. Crates are mapped as `part 'crate' : Crate;` inside each part definition. Temporary dependencies are marked in the model with the stage that removes them. Checked by `tools/check_architecture.py` in CI | `dependency` is the standard KerML/SysML concept for "requires"; connections describe runtime interaction. Explicit edges keep shortcuts around SystemState visible instead of hiding them behind transitivity |
| 2026-09-27 | Studio → LanguageCore and Assistant → LanguageCore allowed explicitly (element identities and model types) | Both already use language identities and types; stating it keeps the rule honest |
| 2026-09-27 | R-3 decided (rebuild) and R-6 adjusted (git2, `model/` + `agentique.json`, merge by identity), both pending Operator confirmation. D-1 reworded: relationships are properties of their owning element whose references carry the target's identity, instead of separate relationship elements | Stage 1 spikes and review (`docs/stages.md`) |

---

## 7. Rules against future drift

### 7.1 Governing documents (the new `AGENTS.md` must say this)

1. **`REALIGNMENT.md` is the governing direction.** Every piece of work names
   the stage (§5) and the scenario step, decision (C-n) or recommendation (R-n)
   it serves. If it serves none, it does not start. Propose an update to this
   document instead.
2. **Work only in the current stage.** Items that wait do not start early,
   however attractive.
3. **Read §1.3 before designing anything.** Prefer the simplest design;
   generalise instead of multiplying parts; use standard names; understand
   intent before acting literally; ask when intent is unclear.
4. **The stable core is locked** (R-16). Changing `LanguageCore`, `SystemState`
   operations or the persistence format requires an explicit Operator decision,
   recorded in §6.
5. **Changes that cut across parts start in the self-model.** Update
   `models/agentique/` first. The dependency check (R-15) must stay green.
6. **No new crate, top-level folder, register, generator or evidence format**
   without a distinct responsibility in the self-model and an Operator-visible
   reason.
7. **Standards are a reference, not a project.** Consult the pinned
   specifications to get concepts right. Extend the subset only for a scenario
   need, and record it in the manifest. Record any deviation in one line with
   its reason.
8. Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`
   and `cargo test --workspace` before handing work back, and report actual
   results honestly, including failures.

### 7.2 One truth per topic

- **Each topic has exactly one current document:** this file (direction and
  decisions), `README.md` (what it is and how to run it), `models/agentique/`
  (architecture), the subset manifest, the deviations list, and short
  per-crate READMEs.
- **When a decision changes, edit the governing text in place** and add a
  one-line entry to §6 (or to a decision log section added here) saying what
  changed, when, and why. Delete or rewrite superseded text. **Never leave
  competing versions side by side**; git history keeps the old one.
- **No versioned copies** of documents, profiles, registers or evidence
  directories (no `-v2`, `-v9`, `phase3`). Git provides versions.
- **Historical documents** are not kept in the working tree. Reference the
  archive tag or the commit instead.
- Any change to a requirement or decision keeps its origin (C-n / R-n) and
  states what it replaces.

### 7.3 How completion is demonstrated

- **A stage is complete when the Operator has used its outcome and accepts
  it** (C-15). Acceptance is recorded in one short stage summary: what was
  shown, what was measured, what failed, what is deferred. The summary lives in
  this document's §5 or in one `docs/stages.md`. It never grows into a
  directory tree of receipts.
- **Automated tests are necessary but never sufficient.** A green test suite
  does not mean a stage is done.
- **Report honestly.** "Works", "partially works", "not tried" and "failed" are
  different statements. A run completing is not a check passing. Never state
  that something works because a lot of code or evidence exists.
- **CI is green on a healthy tree.** A known failure is fixed, or the test is
  removed with a recorded reason. It is never left permanently red.
- **Do not commit** generated logs, receipts, hashes of screenshots or other
  machine-generated evidence dumps.

### 7.4 Naming

- **Model concepts** use KerML/SysML names: part, port, interface, connection,
  item, attribute, requirement, state, action, specialisation, redefinition.
- **Everything else** uses plain, widely understood software terms. Use
  "change", "branch", "commit", "view", "validation error", "undo". Do not use
  "candidate", "World", "publication", "receipt", "frontier", "authority",
  "fabric" or "rematerialization".
- **Product nouns** are fixed by this document: Studio, Surface, Panels,
  Conversation, Assistant, Orchestrator (later), System State, lock, scenario,
  simulation, implementation link, drift.
- A new product term needs a reason that no standard term fits, and an entry
  in the glossary below.

### 7.5 Glossary

| Term | Meaning |
|---|---|
| **Operator** | The person using Agentique |
| **Studio** | The Agentique application |
| **Surface** | The main visual, spatial view of the System State |
| **Panels** | Side panels for detail (inspector, requirements, history, simulation, links) |
| **Conversation** | Chat with the Assistant |
| **Assistant** | The AI agent the Operator works with, with tools and skills; later the Orchestrator |
| **Orchestrator** | The later form of the Assistant that directs other assistants |
| **System State** | The live, authoritative KerML/SysML description of the system being built |
| **Lock** | A mark on a part meaning it changes only with the Operator's confirmation |
| **Subset manifest** | The list of supported, partial and excluded KerML/SysML constructs, with reasons |
| **Deviation** | A recorded place where Agentique intentionally departs from the standard |
| **Scenario (simulation)** | A defined sequence of interactions run against the architecture |
| **Implementation link** | A link from a model element to code, tests or a running service |
| **Drift** | A detected difference between the model and a linked implementation |
| **Archive tag** | `archive/pre-realignment`, the preserved state before Stage 0 |
