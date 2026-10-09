package AgentiqueArchitecture {
    doc /* Agentique's own architecture, and the contract for its code (C-20). Its purpose is ROADMAP §1.1's (C-55), held here as the requirement `purpose`. Each part says what it is for, what it owns, its contract and what it must not do; the crates that implement it are its `Crate` parts; `dependency` lists every allowed use of one part by another; requirements state the guarantees; the scenarios walk the workflows across the parts with their failure paths. Inside: one System State per project, its history in git, the Library, scenarios and runs, implementation links and checks, controlled execution, the Assistant, the builds of Agentique itself, and the Operator's objectives, which the Orchestrator runs (C-53). Outside: a project's code and deployments, other people, and any sandbox the host does not have. The scenarios run in model execution: they check this model's account of the workflows, not the code; the code is checked by the tests linked to each step (model/links.json). */

    private import ScalarValues::*;

    part def Crate {
        doc /* A Cargo package. A part usage typed by Crate and named after the package
             * says that the crate implements the enclosing part.
             */
    }

    enum def Actor {
        doc /* Who made a change: the Operator, or the Assistant (its changes can be
             * undone together).
             */
        enum operator;
        enum assistant;
    }

    enum def Refusal {
        doc /* Why a change was not applied; nothing changed. */
        enum locked;
        enum stale;
        enum invalid;
        enum notSaved;
        enum notVerified;
    }

    enum def Tool {
        doc /* The kinds of the Assistant's tool calls that matter to the workflows:
             * reading changes nothing; applying makes one System State change.
             */
        enum readModel;
        enum applyChanges;
    }

    enum def Started {
        doc /* Which build of Agentique the launcher has running. */
        enum newBuild;
        enum lastKnownGood;
    }

    item def Edit {
        doc /* An Operator's edit on the Surface or in the Inspector, such as renaming a
             * part. `locked` says whether it touches a locked element.
             */
        attribute locked : Boolean;
    }

    item def Change {
        doc /* One System State change: typed operations on elements by identity (create,
             * delete, rename, move, connect, set property, lock, unlock), by an actor,
             * against a base revision, with the locks the Operator confirmed. It
             * applies completely or not at all, and is one undo step.
             */
        attribute madeBy : Actor;
        attribute touchesLock : Boolean;
        attribute confirmed : Boolean;
    }

    item def Applied {
        doc /* The change event: the revision after the change, and what it created,
             * changed and deleted.
             */
        attribute revision : Natural;
    }

    item def Refused {
        doc /* A change was not applied, and why. */
        attribute reason : Refusal;
    }

    item def Documents {
        doc /* The printed text of the documents a change touched, and the identity
             * file, for History to save atomically.
             */
        attribute count : Natural;
    }

    item def Saved {
        doc /* Whether the save is durable (its journal written and the files in
             * place).
             */
        attribute ok : Boolean;
    }

    item def Load {
        doc /* Read the model folder: after a crash, the save journal is finished or
             * discarded first, so the model is the old one or the new one.
             */
        attribute folder : String;
    }

    item def Loaded {
        doc /* The model as last saved, with its identities and locks. */
        attribute count : Natural;
    }

    item def Undo {
        doc /* Undo the last change: the state before it comes back, and is saved. */
        attribute steps : Natural;
    }

    item def Reopen {
        doc /* Open the project again (after closing Agentique). */
        attribute folder : String;
    }

    item def SurfaceUpdate {
        doc /* What the Surface and the Panels show after a change: the revision they
             * now draw.
             */
        attribute revision : Natural;
    }

    item def LockQuestion {
        doc /* The Operator is asked to confirm a change to a locked part. */
        attribute madeBy : Actor;
    }

    item def LockAnswer {
        doc /* The Operator's answer to a lock question. */
        attribute allow : Boolean;
    }

    item def StatusMessage {
        doc /* What the status bar says when something was not done, and why. */
        attribute reason : Refusal;
    }

    item def Intent {
        doc /* A message the Operator writes in the Conversation. */
        attribute text : String;
    }

    item def ToolCall {
        doc /* A tool call from the Assistant's runtime, checked against its schema
             * before the Studio carries it out.
             */
        attribute tool : Tool;
        attribute touchesLock : Boolean;
    }

    item def ToolResult {
        doc /* What a tool call did, as the model reads it. */
        attribute ok : Boolean;
    }

    item def Reply {
        doc /* The Assistant's answer at the end of its turn. */
        attribute changed : Boolean;
    }

    item def Report {
        doc /* The turn as the Conversation shows it: the reply, and whether the model
             * changed (each change can be undone).
             */
        attribute changed : Boolean;
    }

    item def TaskApproval {
        doc /* The Operator approves a task: its outcome, affected parts, protected
             * boundaries, permitted writes and acceptance criteria, and its required
             * checks, fixed from now on. `passing` says, for the walkthrough, how many
             * of them the work will pass.
             */
        attribute required : Natural;
        attribute passing : Natural;
    }

    item def WorktreeRequest {
        doc /* Make the task's worktree on its own branch from the base commit. */
        attribute task : String;
    }

    item def WorktreeReady {
        doc /* The worktree exists; the Operator's working copy was not touched. */
        attribute task : String;
    }

    item def Brief {
        doc /* What the worker is given, taken from the model each time: the parts in
             * scope with their contracts, the scenarios, the protected paths and
             * the rules.
             */
        attribute task : String;
    }

    item def WorkDone {
        doc /* The worker finished (or stopped): the files it changed. */
        attribute files : Natural;
    }

    item def CheckRequest {
        doc /* Run the task's required checks on the task commit. */
        attribute required : Natural;
        attribute passing : Natural;
    }

    item def CheckResults {
        doc /* Every required check with an explicit outcome: how many passed. */
        attribute required : Natural;
        attribute passed : Natural;
    }

    item def Verification {
        doc /* The one summary: passed only when every required check passed and nothing
             * else failed; it names the commit it checked.
             */
        attribute passed : Boolean;
    }

    item def Review {
        doc /* The task for the Operator to review: the model changes and the code
             * changes together, each required check with its outcome.
             */
        attribute passed : Boolean;
    }

    item def IntegrateRequest {
        doc /* The Operator integrates the reviewed task, or, after seeing what was not
             * verified, integrates it without a passing verification.
             */
        attribute withoutVerification : Boolean;
    }

    item def Integrate {
        doc /* Bring exactly the reviewed task commit into the project's branch. */
        attribute task : String;
    }

    item def Integrated {
        doc /* The branch is at the task commit; nothing else in the working copy
             * changed.
             */
        attribute ok : Boolean;
    }

    item def BuildRequest {
        doc /* Build Agentique from the integrated commit into a versioned folder.
             * `builds` says, for the walkthrough, whether it compiles.
             */
        attribute builds : Boolean;
    }

    item def Built {
        doc /* A build with its manifest: source commit, toolchain, checks, digests. */
        attribute ok : Boolean;
    }

    item def AdoptRequest {
        doc /* Use this build: the launcher starts it. `starts` says, for the
             * walkthrough, whether it starts and reports ready.
             */
        attribute starts : Boolean;
    }

    item def Running {
        doc /* Which build runs after adoption: the new one, or the last known good one
             * when the new one did not start.
             */
        attribute build : Started;
    }

    item def ObjectiveRequest {
        doc /* The Operator's objective (C-53), or a child objective an agent delegated
             * within its parent's permissions and budgets (C-54): its intent, budgets
             * and the permissions it carries. `cycles` is its cycle budget; `explore`
             * says whether its cycles start by exploring the running application;
             * `depth` is 0 for the Operator's and one more than its parent's for a
             * child (at most 2).
             */
        attribute cycles : Natural;
        attribute explore : Boolean;
        attribute depth : Natural;
    }

    item def Directive {
        doc /* One agent's instruction to another (C-54): its author, recipient,
             * parent objective, scope and status; created only by a tool the
             * Orchestrator validates, shown in the objective's thread. `delegates`
             * says whether it starts a child objective, rather than handing work to
             * another role within a cycle.
             */
        attribute delegates : Boolean;
        attribute refused : Boolean {
            doc /* Whether the Orchestrator refused it (a child beyond the parent's depth, permissions or budgets), with the reason in the thread. */
        }
    }

    item def CycleReport {
        doc /* One cycle's outcome as the Objectives panel shows it: the findings its
             * exploration reproduced (C-54), whether its change was merged and
             * whether its build was adopted.
             */
        attribute reproduced : Natural;
        attribute merged : Boolean;
        attribute adopted : Boolean;
    }

    item def Observation {
        doc /* The Studio's visible state as text (C-53): its identity, screen, dialog,
             * selection, controls and commands, at a revision.
             */
        attribute revision : Natural;
    }

    item def ControlAction {
        doc /* An agent's action on the Studio: a command, input on a control, or a
             * selection, naming the agent, the identity and revision it was observed
             * against, and its goal and reason for observer mode (C-54).
             */
        attribute observedAt : Natural;
    }

    item def ActionOutcome {
        doc /* Whether the action went through the Studio's own handlers, or was
             * refused (stale, gone, disabled, paused) and why.
             */
        attribute ok : Boolean;
    }

    port def OperatorPort {
        doc /* The Operator's side of the Studio: edits, undo and opening; messages to
             * the Assistant; tasks, builds and adoption; objectives and their cycles
             * (C-53); and what the Studio shows.
             */
        in item edit : Edit;
        in item undo : Undo;
        in item reopen : Reopen;
        in item answer : LockAnswer;
        in item intent : Intent;
        in item approval : TaskApproval;
        in item integrate : IntegrateRequest;
        in item build : BuildRequest;
        in item adopt : AdoptRequest;
        in item goal : ObjectiveRequest;
        out item update : SurfaceUpdate;
        out item question : LockQuestion;
        out item status : StatusMessage;
        out item report : Report;
        out item review : Review;
        out item integrated : Integrated;
        out item built : Built;
        out item running : Running;
        out item cycle : CycleReport;
    }

    port def ChangePort {
        doc /* The System State's one change path: changes, undo and opening in; a
             * change event or a refusal out.
             */
        in item change : Change;
        in item undo : Undo;
        in item reopen : Reopen;
        out item applied : Applied;
        out item refused : Refused;
    }

    port def FolderPort {
        doc /* History's side: documents to save and loads in; saved and loaded out. */
        in item documents : Documents;
        in item load : Load;
        out item saved : Saved;
        out item loaded : Loaded;
    }

    port def TurnPort {
        doc /* The Assistant's turn: the Operator's message and tool results in; tool
             * calls and the reply out.
             */
        in item intent : Intent;
        in item result : ToolResult;
        in item brief : Brief;
        out item call : ToolCall;
        out item reply : Reply;
        out item done : WorkDone;
    }

    port def JobPort {
        doc /* Execution's jobs: worktrees, checks, integration and builds. */
        in item worktree : WorktreeRequest;
        in item checks : CheckRequest;
        in item integrate : Integrate;
        in item build : BuildRequest;
        out item ready : WorktreeReady;
        out item results : CheckResults;
        out item integrated : Integrated;
        out item built : Built;
    }

    port def VerificationPort {
        doc /* Implementation's verdict on a task's check results. */
        in item results : CheckResults;
        out item outcome : Verification;
    }

    port def LaunchPort {
        doc /* The launcher: adopt a build, and which build runs. */
        in item adopt : AdoptRequest;
        out item running : Running;
    }

    port def ControlPort {
        doc /* The Studio's control interface (C-53): actions in; observations and the
             * outcome of each action out.
             */
        in item request : ControlAction;
        out item observation : Observation;
        out item outcome : ActionOutcome;
    }

    port def ObjectivePort {
        doc /* The Orchestrator's side: objectives in; directives and cycle reports out,
             * for the objective's thread in the Conversation and the dashboard.
             */
        in item goal : ObjectiveRequest;
        out item directive : Directive;
        out item cycle : CycleReport;
    }

    part def LanguageCore {
        doc /* The KerML/SysML subset: one element tree, text parse and print, name
             * resolution, typing, specialisation, behaviour, expressions and
             * validation. Owns the meaning and validity of a model (it knows no UI,
             * storage or AI). Pure functions the System State, the Library and the
             * runs call. Depends on no other part. Locked core (R-16).
             */
        part 'agq-language' : Crate {
            doc /* The language core: the SysML subset as an element tree with parse,
                 * print and validate (Stage 1, R-3 rebuild).
                 */
        }
    }

    part def SystemState {
        doc /* The live model of one project and its one change path. Owns the element
             * tree, the locks, the revision and the undo steps (in memory; saved
             * through History). Contract: a change applies completely or is refused
             * (stale, locked, invalid) and leaves everything as it was; a change that
             * cannot be saved is undone and said; a well-formed change that makes the
             * model invalid is applied and its problems shown (R-18). It does not
             * decide intent, and nothing else changes the model. Locked core (R-16).
             */
        part 'agq-system-state' : Crate;
        port changes : ChangePort;
        port storage : ~FolderPort;
        attribute revision : Natural = 0 {
            doc /* Counts changes since the project was opened; a change prepared against
                 * an older revision is stale.
                 */
        }
        exhibit state live {
            entry;
            then ready;
            state ready;
            state saving;
            state undoing;
            state opening;
            transition refuseLocked first ready accept c : Change via changes if c.touchesLock and not c.confirmed do send new Refused(reason = Refusal::locked) via changes then ready;
            transition apply first ready accept c : Change via changes if not c.touchesLock or c.confirmed do send new Documents(count = 1) via storage then saving;
            transition applied first saving accept s : Saved via storage if s.ok do action {
                assign revision := revision + 1;
                then send new Applied(revision = revision) via changes;
            } then ready;
            transition notSaved first saving accept s : Saved via storage if not s.ok do send new Refused(reason = Refusal::notSaved) via changes then ready;
            transition undo first ready accept u : Undo via changes do send new Documents(count = 1) via storage then undoing;
            transition undone first undoing accept s : Saved via storage if s.ok do action {
                assign revision := revision + 1;
                then send new Applied(revision = revision) via changes;
            } then ready;
            transition reopen first ready accept o : Reopen via changes do send new Load(folder = o.folder) via storage then opening;
            transition opened first opening accept l : Loaded via storage do action {
                assign revision := 0;
                then send new Applied(revision = 0) via changes;
            } then ready;
        }
    }

    part def History {
        doc /* A project's model folder on disk and its history in git. Owns the model
             * files, the save journal, the identity file and the checkpoints. Contract:
             * a save is durable before it is reported (journal, atomic renames); after
             * a crash the next open finishes or discards it, never mixing old and new;
             * a model file edited outside Agentique is never overwritten. Knows no
             * SysML. The persistence format is locked core (R-16).
             */
        part 'agq-history' : Crate {
            doc /* The model folder of a project: atomic saves of the text files and the
                 * identity and lock file, commits, branches and reading any version,
                 * through an embedded git (R-6).
                 */
        }
        port folder : FolderPort;
        exhibit state keeping {
            entry;
            then ready;
            state ready;
            transition save first ready accept d : Documents via folder do send new Saved(ok = true) via folder then ready;
            transition load first ready accept l : Load via folder do send new Loaded(count = 1) via folder then ready;
        }
    }

    part def Library {
        doc /* The Library of building blocks (C-49, ROADMAP §4.13): finds, describes and
             * searches reusable definitions (the built-in blocks, the project's own, My
             * Library), works out what a block needs, and plans its use, specialisation,
             * overrides and extraction as ordinary System State changes, the same for the
             * Studio and the Assistant. Owns the built-in blocks and My Library's file.
             * Meaning and validity stay in LanguageCore; applying stays in SystemState.
             */
        part 'agq-library' : Crate;
    }

    part def Studio {
        doc /* The application the Operator works in: the Surface, the Panels, the
             * Conversation, Settings, the task review and the Objectives panel. It
             * carries out every edit and every tool call through the System State's one
             * change path (asking about locks), runs tasks through Execution, and decides
             * nothing the Operator or an objective has not. Its control interface (C-53)
             * gives agents what it shows as an observation, every interactive control
             * with a readable label, and carries out their actions through the same
             * handlers as the Operator's input, refusing stale ones and actions by an
             * agent that does not hold the window (C-54); observer mode draws each
             * agent's typing, clicks, focus and scrolling with its goal and decisions,
             * at the Operator's speed; in a test instance agents may also operate the
             * Conversation and undo, and everything else the Operator's own stays
             * refused to them. The Conversation is the one place for intent and every
             * objective's thread (the Operator's messages, agents' directives and
             * results, system events), and the Objectives panel a dashboard of the same
             * records, both through the same commands. Settings name each agent role's
             * model, effort and fallback, and the Studio resolves them before an
             * objective starts (C-54).
             * Owns presentation (cameras, layouts, selection), the session, Settings,
             * the control interface's event trace and each project's app data
             * (conversation, jobs, runs, required checks).
             */
        part 'agq-studio-native' : Crate;
        part 'agq-studio-scene' : Crate;
        port operator : OperatorPort;
        port control : ControlPort;
        port objectives : ~ObjectivePort;
        port edits : ~ChangePort;
        port assistant : ~TurnPort;
        port execution : ~JobPort;
        port verifying : ~VerificationPort;
        port launcher : ~LaunchPort;
        attribute required : Natural = 0 {
            doc /* The task's required checks, fixed when the Operator approved it. */
        }
        attribute passing : Natural = 0 {
            doc /* For the walkthrough: how many of them the work will pass. */
        }
        attribute verified : Boolean = false {
            doc /* Whether the task's verification passed. */
        }
        exhibit state working {
            entry;
            then idle;
            state idle;
            state applying;
            state asking;
            state thinking;
            state preparing;
            state implementing;
            state checking;
            state judging;
            state reviewing;
            state integrating;
            state building;
            state adopting;
            transition edit first idle accept e : Edit via operator do action {
                assign requester := Actor::operator;
                then send new Change(madeBy = Actor::operator, touchesLock = e.locked, confirmed = false) via edits;
            } then applying;
            transition shown first applying accept a : Applied via edits if requester == Actor::operator do send new SurfaceUpdate(revision = a.revision) via operator then idle;
            transition askLock first applying accept r : Refused via edits if r.reason == Refusal::locked do send new LockQuestion(madeBy = requester) via operator then asking;
            transition notDone first applying accept r : Refused via edits if r.reason != Refusal::locked and requester == Actor::operator do send new StatusMessage(reason = r.reason) via operator then idle;
            transition confirmed first asking accept a : LockAnswer via operator if a.allow do send new Change(madeBy = requester, touchesLock = true, confirmed = true) via edits then applying;
            transition keptLocked first asking accept a : LockAnswer via operator if not a.allow and requester == Actor::operator do send new StatusMessage(reason = Refusal::locked) via operator then idle;
            transition undo first idle accept u : Undo via operator do action {
                assign requester := Actor::operator;
                then send u via edits;
            } then applying;
            transition reopen first idle accept o : Reopen via operator do action {
                assign requester := Actor::operator;
                then send o via edits;
            } then applying;
            transition ask first idle accept i : Intent via operator do send i via assistant then thinking;
            transition read first thinking accept c : ToolCall via assistant if c.tool == Tool::readModel do send new ToolResult(ok = true) via assistant then thinking;
            transition assistantChange first thinking accept c : ToolCall via assistant if c.tool == Tool::applyChanges do action {
                assign requester := Actor::assistant;
                then send new Change(madeBy = Actor::assistant, touchesLock = c.touchesLock, confirmed = false) via edits;
            } then applying;
            transition assistantApplied first applying accept a : Applied via edits if requester == Actor::assistant do action {
                send new SurfaceUpdate(revision = a.revision) via operator;
                then send new ToolResult(ok = true) via assistant;
            } then thinking;
            transition assistantRefused first applying accept r : Refused via edits if r.reason != Refusal::locked and requester == Actor::assistant do send new ToolResult(ok = false) via assistant then thinking;
            transition assistantKeptLocked first asking accept a : LockAnswer via operator if not a.allow and requester == Actor::assistant do send new ToolResult(ok = false) via assistant then thinking;
            transition reported first thinking accept r : Reply via assistant do send new Report(changed = r.changed) via operator then idle;
            transition approve first idle accept a : TaskApproval via operator do action {
                assign required := a.required;
                then assign passing := a.passing;
                then send new WorktreeRequest(task = "task") via execution;
            } then preparing;
            transition brief first preparing accept w : WorktreeReady via execution do send new Brief(task = w.task) via assistant then implementing;
            transition check first implementing accept d : WorkDone via assistant do send new CheckRequest(required = required, passing = passing) via execution then checking;
            transition judge first checking accept r : CheckResults via execution do send r via verifying then judging;
            transition review first judging accept v : Verification via verifying do action {
                assign verified := v.passed;
                then send new Review(passed = v.passed) via operator;
            } then reviewing;
            transition integrate first reviewing accept i : IntegrateRequest via operator if verified or i.withoutVerification do send new Integrate(task = "task") via execution then integrating;
            transition notIntegrated first reviewing accept i : IntegrateRequest via operator if not verified and not i.withoutVerification do send new StatusMessage(reason = Refusal::notVerified) via operator then reviewing;
            transition integrated first integrating accept i : Integrated via execution do send i via operator then idle;
            transition build first idle accept b : BuildRequest via operator do send b via execution then building;
            transition built first building accept b : Built via execution do send b via operator then idle;
            transition adopt first idle accept a : AdoptRequest via operator do send a via launcher then adopting;
            transition running first adopting accept r : Running via launcher do send r via operator then idle;
        }
        attribute requester : Actor = Actor::operator {
            doc /* Who asked for the change in progress: the Operator by an edit, or the Assistant by a tool call. One change path and one lock question serve both; only where the answer goes differs. */
        }
    }

    part def Assistant {
        doc /* The AI agent the Operator works with (C-5): its turn runs on a runtime,
             * either Agentique's own loop over a provider or the Claude Agent runtime,
             * and every tool call it makes is checked against its schema and carried
             * out by the Studio on the one change path; in an implementation task it is
             * the worker, with the coding tools in the task's worktree, and in an
             * objective its sessions are the Orchestrator's lead, implementer, reviewer
             * and evaluator, each on its role's model (C-53, C-54). Owns the tool
             * definitions, the skills, the
             * development sessions' permission policies and the conversation's entries.
             * Its behaviour comes from a model, so the scenarios stand it in. It never
             * changes the model except through Agentique's tools, and works on files
             * and commands only inside its session's permission policy.
             */
        part 'agq-assistant' : Crate;
        port turn : TurnPort;
    }

    part def ClaudeAgentRuntime {
        doc /* The Claude Agent runtime (C-51, C-53, ROADMAP §4.7): a TypeScript
             * companion process (claude-agent/) that runs the official Claude Agent SDK's
             * loop for the Assistant and the Orchestrator's agents, speaking protocol 2
             * with the Studio on standard input and output. Owns the SDK's session files
             * (in its own configuration folder). Contract: the SDK's own tools work
             * under the permission policy the Studio gives each session, enforced in one
             * pre-tool hook (what it allows runs, what it forbids is refused with the
             * reason, anything else goes to the Studio); a model file is never written
             * with a file tool, so the model changes only through Agentique's tools,
             * served by one MCP server whose calls the Studio carries out; the project's
             * configuration is loaded, never the machine's user settings; the
             * environment carries no key but its model's, and a session runs only on
             * the credential the Studio gave it (a key, or the Operator's own
             * subscription token): the source the SDK reports is checked before the
             * first prompt, and a session that would use another, such as the
             * machine's claude.ai login, stops (C-54). It can report whether a
             * local Claude login exists, never its token. Not a sandbox: it runs with
             * the Operator's rights and reaches its model's API (Anthropic, or an
             * Anthropic-compatible endpoint). No crate: its code is in claude-agent/src.
             * A safeguard: locked.
             */
    }

    part def Providers {
        doc /* Talks to model providers through rig (and a thin Jev client, C-34);
             * keys in the OS credential store; model lists, capabilities, usage. The
             * only part that uses rig, tokio, reqwest or the credential-store crates
             * (R-21, R-41). Owns the keys, the capability table (which models chat
             * and which make typed decisions, which decision models are known
             * pinned versions, each model's dated price) and every provider call:
             * streamed chat, and typed decisions (C-35, C-52) with replies read completely within a size
             * limit and checked against the request that was sent, usage that says
             * when it is unknown, one deadline for every attempt and wait, and
             * cancellation that sends nothing more. Exposes no rig, tokio or HTTP
             * type. Temporary: agq-assistant still uses reqwest for its hand-written
             * Claude client until W5.7 moves it here.
             */
        part 'agq-providers' : Crate {
            doc /* The provider layer: one streamed model call for every provider,
                 * the capability table, keys and usage (S4.2 kept, start of W5.7).
                 */
        }
    }

    part def Simulation {
        doc /* Runs scenarios (C-50, ROADMAP §4.14): compiles a scenario and a fixed model
             * snapshot into disposable runtime structures and executes them with
             * stand-ins, recordings or a live model client it is given; records traces,
             * check verdicts and results with provenance, and works out freshness. Owns
             * run results (app data). It gives a live model client each call's
             * deadline (plain std types, never stored) and carries, hashes and records
             * the binding the Studio prepared (which provider and model answer, through
             * which adapter and mapping), so a recording answers only the request it
             * was made for (C-52). It never changes the System State and never
             * reaches a provider or any other service by itself.
             */
        part 'agq-simulation' : Crate;
    }

    part def Implementation {
        doc /* What the model says about code (C-50, ROADMAP §4.15): implementation links,
             * code-to-model lookup, the implementation runner that drives real code
             * through the project's harness, the supported checks (dependency
             * boundaries, mapped contract shapes, linked tests), whose failures are
             * drift, and a task's required checks and verification. Owns
             * model/links.json (saved through History). Contract: a verification passes
             * only when every required check passed and nothing else failed; a check
             * that did not run is never a pass. Its processes run through Execution.
             * A safeguard: locked.
             */
        part 'agq-implementation' : Crate;
        port verdict : VerificationPort;
        exhibit state judging {
            entry;
            then ready;
            state ready;
            transition everyRequiredCheckPassed first ready accept r : CheckResults via verdict if r.passed == r.required do send new Verification(passed = true) via verdict then ready;
            transition aRequiredCheckDidNotPass first ready accept r : CheckResults via verdict if r.passed < r.required do send new Verification(passed = false) via verdict then ready;
        }
    }

    part def Execution {
        doc /* Controlled side effects outside the System State (C-50, ROADMAP §4.15):
             * repository reads, scoped writes in a worktree, allow-listed commands with
             * scrubbed environments, worktrees, the task commit and its integration, as
             * jobs with journals that can be cancelled and resumed without repeating a
             * completed side effect. Owns the job journals and the worktrees. Contract:
             * protected paths are never written, whatever their spelling; integration
             * brings exactly the reviewed commit and touches nothing else. It reports
             * the isolation the host really offers: trusted-local, no sandbox. A
             * safeguard: locked.
             */
        part 'agq-execution' : Crate;
        port jobs : JobPort;
        exhibit state running {
            entry;
            then idle;
            state idle;
            transition makeWorktree first idle accept w : WorktreeRequest via jobs do send new WorktreeReady(task = w.task) via jobs then idle;
            transition runChecks first idle accept c : CheckRequest via jobs do send new CheckResults(required = c.required, passed = c.passing) via jobs then idle;
            transition integrate first idle accept i : Integrate via jobs do send new Integrated(ok = true) via jobs then idle;
            transition build first idle accept b : BuildRequest via jobs do send new Built(ok = b.builds) via jobs then idle;
        }
    }

    part def Launcher {
        doc /* Starts the current build of Agentique and keeps the last known good one
             * (C-51, ROADMAP §4.15): if a new build does not start and report ready, it
             * starts the last known good build, which says what happened. As the
             * supervisor (C-53) it stays the Studio's parent: it starts the build a
             * handover names, restarts a crashed Studio once and then falls back, and
             * makes a build last known good only after its check after adoption. Owns
             * the builds registry. Depends on no other part, so it works when a new build
             * does not; it never builds, downloads or deletes. A safeguard: locked.
             */
        part 'agq-launcher' : Crate;
        port launch : LaunchPort;
        exhibit state launching {
            entry;
            then ready;
            state ready;
            transition startsNew first ready accept a : AdoptRequest via launch if a.starts do send new Running(build = Started::newBuild) via launch then ready;
            transition fallsBack first ready accept a : AdoptRequest via launch if not a.starts do send new Running(build = Started::lastKnownGood) via launch then ready;
        }
    }

    part def Orchestrator {
        doc /* Runs the Operator's objectives (C-53, C-54, ROADMAP §4.16). It is composed of its deterministic Cycle, which decides each next phase from recorded results, and its role agents (Explorer, Lead, Implementer, Reviewer), each a session of the Assistant's runtime on its role's model; it refers to the test instances its explorer and evaluator operate, other builds of Agentique, without containing them. Agents hand work to each other only as directives it validates and records, and child objectives run within their parent's permissions and budgets. Owns the objectives' records (app data: state, journal, sessions, effective models, results, spend per role, directives, the thread, continuation) and each project's testing knowledge (coverage, findings with their replays and dispositions, runs). Contract: a finding is a deterministic check that failed and reproduced; a cycle merges only when every required check and acceptance criterion passed on the reviewed commit, one criterion showed the defect on the original implementation, a user-facing change was evaluated in a test instance, the reviewer approved, the repository's checks passed and the change leaves ROADMAP.md and the purpose alone; it adopts only after its test instance passed; a check that did not run is never a pass; weakening a check never counts; budgets and lack of progress stop it; resuming never repeats a completed side effect; Stop interrupts every session. A safeguard: locked. */
        port objectives : ObjectivePort;
        part 'agq-orchestrator' : Crate;
        port jobs : ~JobPort;
        port launch : ~LaunchPort;
        part cycle : Cycle;
        part explorer : Explorer;
        part lead : Lead;
        part implementer : Implementer;
        part reviewer : Reviewer;
        connect objectives to cycle.objectives;
        connect jobs to cycle.jobs;
        connect launch to cycle.launch;
        connect cycle.explorer to explorer.goal;
        connect cycle.lead to lead.work;
        connect cycle.implementer to implementer.work;
        connect cycle.reviewer to reviewer.review;
    }

    part def Agentique {
        doc /* The product and its parts, connected as they work together. The Operator
             * reaches it through the Studio.
             */
        port operator : OperatorPort;
        part languageCore : LanguageCore;
        part systemState : SystemState;
        part history : History;
        part 'library' : Library;
        part studio : Studio;
        part assistant : Assistant;
        part claudeAgent : ClaudeAgentRuntime;
        part providers : Providers;
        part simulation : Simulation;
        part implementation : Implementation;
        part execution : Execution;
        part launcher : Launcher;
        part orchestrator : Orchestrator;
        connect operator to studio.operator;
        connect studio.edits to systemState.changes;
        connect systemState.storage to history.folder;
        connect studio.assistant to assistant.turn;
        connect studio.execution to execution.jobs;
        connect studio.verifying to implementation.verdict;
        connect studio.launcher to launcher.launch;
        connect studio.objectives to orchestrator.objectives;
    }

    dependency from Studio to SystemState;

    dependency from Studio to Assistant {
        doc /* The Conversation in the Studio sends the Operator's messages to the
             * Assistant and shows its replies (refines R-14).
             */
    }

    dependency from Studio to LanguageCore {
        doc /* Element identities and model types shown on the Surface. */
    }

    dependency from Studio to Providers {
        doc /* Settings tests keys and lists models; a live evaluation's model client
             * (the Studio's, handed to Simulation) asks the capability table how an
             * agent's model is called and calls it through Providers (C-52).
             */
    }

    dependency from Studio to Library {
        doc /* The Library panel, insertion, previews and My Library. */
    }

    dependency from Assistant to SystemState;

    dependency from Assistant to LanguageCore {
        doc /* Element identities and model types in tool calls. */
    }

    dependency from Assistant to Providers {
        doc /* Model calls: requests, streamed replies, usage; and the key of the Claude
             * Agent runtime's model (Anthropic's, or the provider whose
             * Anthropic-compatible endpoint it uses, C-53) for its process.
             */
    }

    dependency from Assistant to Library {
        doc /* The Library tools: search, read and use building blocks. */
    }

    dependency from Assistant to ClaudeAgentRuntime {
        doc /* The Claude Agent runtime: started per turn, protocol 2 (C-51, C-53). */
    }

    dependency from Studio to Simulation {
        doc /* The Scenarios tab, run controls, traces and results (C-50). */
    }

    dependency from Studio to Implementation {
        doc /* Implementation links, checks, drift and patch review (C-50). */
    }

    dependency from Studio to Execution {
        doc /* Jobs the Studio starts, cancels and resumes; trusted-local mode (C-50). */
    }

    dependency from Studio to Launcher {
        doc /* Builds, trying and adopting: the registry and the manifests (C-51). */
    }

    dependency from Assistant to Simulation {
        doc /* The run tools prepare typed run requests and read results (C-50). */
    }

    dependency from Assistant to Implementation {
        doc /* The code-link tools and the implementation task context (C-50). */
    }

    dependency from Assistant to Execution {
        doc /* The implementation tools prepare typed operations; the Studio carries
             * them out (C-50).
             */
    }

    dependency from Library to SystemState {
        doc /* Plans are System State changes, tried on a copy of the state. */
    }

    dependency from Library to LanguageCore {
        doc /* Blocks are definitions; their features and fit come from Semantics. */
    }

    dependency from SystemState to LanguageCore;

    dependency from SystemState to History;

    dependency from Simulation to LanguageCore {
        doc /* Scenarios, behaviour and expressions are model elements; runs read a
             * snapshot of the tree.
             */
    }

    dependency from Implementation to LanguageCore {
        doc /* Links name elements by identity; contract shapes come from item and enum
             * definitions.
             */
    }

    dependency from Implementation to Simulation {
        doc /* The implementation runner shares the run contract, the step interpreter
             * and the check evaluator with model execution.
             */
    }

    dependency from Implementation to Execution {
        doc /* Builds, tests and the harness run as controlled processes. */
    }

    requirement def OneChangePath {
        doc /* The Surface, the Inspector, the Library and the Assistant change the model
             * only through System State changes, each one undo step; a change to a
             * locked part applies only after the Operator confirms that change, or
             * within an objective that names it (C-53).
             */
        subject systemState : SystemState;
    }

    requirement def SavedBeforeShown {
        doc /* A change is durable before the Surface shows it as applied; one that cannot
             * be saved is undone and said; reopening shows the model as last saved.
             */
        subject systemState : SystemState;
    }

    requirement def AssistantThroughTools {
        doc /* The Assistant changes the model only through Agentique's tools, each
             * checked and carried out by the Studio on the one change path, whatever
             * runtime runs its turn; in a development session its other side effects
             * happen only inside the session's permission policy (C-53).
             */
        subject studio : Studio;
    }

    requirement def RequiredChecksDecide {
        doc /* A task's required checks are fixed when it is approved; it is integrated
             * only with a current, passing verification of all of them, or by the
             * Operator's explicit choice, recorded with the task.
             */
        subject implementation : Implementation;
    }

    requirement def RecoverableAdoption {
        doc /* A build that does not start leaves the last known good build running. */
        subject launcher : Launcher;
    }

    requirement def GatesDecide {
        doc /* Within an objective, a change is merged only when every required check and
             * acceptance criterion passed on the reviewed commit, an independent review
             * approved it and the repository's own checks passed, and a build is adopted
             * only after its test instance passed; an agent's judgment never overrides
             * a failing check (C-53).
             */
        subject orchestrator : Orchestrator;
    }

    requirement oneChangePath : OneChangePath;

    requirement savedBeforeShown : SavedBeforeShown;

    requirement assistantThroughTools : AssistantThroughTools;

    requirement requiredChecksDecide : RequiredChecksDecide;

    requirement recoverableAdoption : RecoverableAdoption;

    requirement gatesDecide : GatesDecide;

    part agentique : Agentique;

    satisfy oneChangePath by agentique.systemState;

    satisfy savedBeforeShown by agentique.systemState;

    satisfy assistantThroughTools by agentique.studio;

    satisfy requiredChecksDecide by agentique.implementation;

    satisfy recoverableAdoption by agentique.launcher;

    satisfy gatesDecide by agentique.orchestrator;

    dependency from Studio to Orchestrator {
        doc /* The Conversation and the Objectives panel start, show, steer and stop
             * objectives; the control interface's types are the Orchestrator's, which
             * drives test instances.
             */
    }

    dependency from Orchestrator to Assistant {
        doc /* Each role is an agent session on the Assistant's runtime. */
    }

    dependency from Orchestrator to Execution {
        doc /* Worktrees, commits and commands; pushing and pull requests as exact
             * commands on Execution's existing allow-list.
             */
    }

    dependency from Orchestrator to Implementation {
        doc /* The required checks and the one verification summary. */
    }

    dependency from Orchestrator to Providers {
        doc /* Typed decisions in operation, and prices for the spend budget. */
    }

    dependency from Orchestrator to Launcher {
        doc /* Builds' manifests and the registry, for trial and adoption. */
    }

    requirement def FindingsReproduce {
        doc /* A finding is recorded only when a deterministic check failed during
             * exploration (an invariant of the running application, or an expectation
             * stated before the action) and its replay from a fresh start of the same
             * build fails the same way; a model's opinion alone is never a finding
             * (C-54).
             */
        subject orchestrator : Orchestrator;
    }

    requirement def DefectShownBefore {
        doc /* A cycle's change counts only when one of its frozen criteria fails on the
             * original implementation by its behaviour (a replay or observation on the
             * base build, or a test that compiles and runs there) and passes on the
             * change; no tests, a compile error or a broken setup on the base is no
             * evidence; a change to user-facing code is evaluated in a test instance
             * (C-54).
             */
        subject orchestrator : Orchestrator;
    }

    requirement def ChildWorkBounded {
        doc /* Work an agent delegates has at most its parent's permissions, counts
             * against the parent's budgets, nests at most two deep and is started only
             * as a directive the Orchestrator validated and recorded; an agent never
             * grants itself a permission, answers the Operator's approvals or speaks as
             * the Operator (C-54).
             */
        subject orchestrator : Orchestrator;
    }

    requirement def OnlyGivenCredentials {
        doc /* A Claude Agent runtime session uses only the credential the Studio gave
             * it; one that would use another, such as the machine's claude.ai login,
             * stops before its first model call (C-54).
             */
        subject claudeAgent : ClaudeAgentRuntime;
    }

    requirement findingsReproduce : FindingsReproduce;

    requirement defectShownBefore : DefectShownBefore;

    requirement childWorkBounded : ChildWorkBounded;

    requirement onlyGivenCredentials : OnlyGivenCredentials;

    satisfy findingsReproduce by agentique.orchestrator;

    satisfy defectShownBefore by agentique.orchestrator;

    satisfy childWorkBounded by agentique.orchestrator;

    satisfy onlyGivenCredentials by agentique.claudeAgent;

    verification def RenameAPart {
        doc /* An ordinary edit: the Operator renames a part; the System State applies the
             * change and History saves it before the Surface shows it; undo brings the
             * old name back, saved too; closing and reopening shows the model as last
             * saved.
             */
        subject product : Agentique;
        objective {
            verify oneChangePath;
            verify savedBeforeShown;
        }
        send new Edit(locked = false) via product.operator;
        then accept renamed : SurfaceUpdate via product.operator;
        then assert constraint appliedAndSaved {
            renamed.revision == 1
        }
        then send new Undo(steps = 1) via product.operator;
        then accept undone : SurfaceUpdate via product.operator;
        then assert constraint undoIsAChangeToo {
            undone.revision == 2
        }
        then send new Reopen(folder = "project") via product.operator;
        then accept reopened : SurfaceUpdate via product.operator;
        then assert constraint reopenedAsSaved {
            reopened.revision == 0
        }
    }

    verification def RenameALockedPart {
        doc /* A failure path of the ordinary edit: the part is locked. The change is
             * refused until the Operator confirms it; refused, nothing changes.
             */
        subject product : Agentique;
        objective {
            verify oneChangePath;
        }
        send new Edit(locked = true) via product.operator;
        then accept asked : LockQuestion via product.operator;
        then send new LockAnswer(allow = false) via product.operator;
        then accept kept : StatusMessage via product.operator;
        then assert constraint nothingChanged {
            kept.reason == Refusal::locked
        }
        then send new Edit(locked = true) via product.operator;
        then accept again : LockQuestion via product.operator;
        then send new LockAnswer(allow = true) via product.operator;
        then accept renamed : SurfaceUpdate via product.operator;
        then assert constraint appliedOnceConfirmed {
            renamed.revision == 1
        }
    }

    verification def AChangeThatCannotBeSaved {
        doc /* A failure path of the ordinary edit: the save fails (a full disk, a model
             * file edited outside Agentique). The change is not applied, and the status
             * bar says so.
             */
        subject product : Agentique;
        objective {
            verify savedBeforeShown;
        }
        part saveFails : Scenarios::StandIn {
            :>> target = product.history;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new Saved(ok = false);
        }
        send new Edit(locked = false) via product.operator;
        then accept status : StatusMessage via product.operator;
        then assert constraint notAppliedAndSaid {
            status.reason == Refusal::notSaved
        }
    }

    verification def TheAssistantChangesTheModel {
        doc /* An Assistant action: the Operator asks; the Assistant reads the model, then
             * changes a locked part; the Studio asks the Operator, who allows it; the
             * change goes through the one change path; the Assistant reports what it
             * did. The Assistant's decisions come from a model, so they are stood in.
             */
        subject product : Agentique;
        objective {
            verify assistantThroughTools;
            verify oneChangePath;
        }
        part readsTheModel : Scenarios::StandIn {
            :>> target = product.assistant;
            :>> call = 1;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new ToolCall(tool = Tool::readModel, touchesLock = false);
        }
        part changesALockedPart : Scenarios::StandIn {
            :>> target = product.assistant;
            :>> call = 2;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new ToolCall(tool = Tool::applyChanges, touchesLock = true);
        }
        part reportsTheChange : Scenarios::StandIn {
            :>> target = product.assistant;
            :>> call = 3;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new Reply(changed = true);
        }
        send new Intent(text = "Rename the store to Warehouse.") via product.operator;
        then accept question : LockQuestion via product.operator;
        then assert constraint askedAboutTheLock {
            question.madeBy == Actor::assistant
        }
        then send new LockAnswer(allow = true) via product.operator;
        then accept shown : SurfaceUpdate via product.operator;
        then accept report : Report via product.operator;
        then assert constraint changedThroughTheChangePath {
            shown.revision == 1 and report.changed
        }
    }

    verification def TheAssistantIsRefusedALock {
        doc /* A failure path of the Assistant action: the Operator keeps the lock; the
             * Assistant is told, changes nothing and says so.
             */
        subject product : Agentique;
        objective {
            verify assistantThroughTools;
        }
        part changesALockedPart : Scenarios::StandIn {
            :>> target = product.assistant;
            :>> call = 1;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new ToolCall(tool = Tool::applyChanges, touchesLock = true);
        }
        part reportsNothing : Scenarios::StandIn {
            :>> target = product.assistant;
            :>> call = 2;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new Reply(changed = false);
        }
        send new Intent(text = "Rename the store.") via product.operator;
        then accept question : LockQuestion via product.operator;
        then send new LockAnswer(allow = false) via product.operator;
        then accept report : Report via product.operator;
        then assert constraint nothingChanged {
            not report.changed
        }
    }

    verification def ADevelopmentTask {
        doc /* A development task: the Operator approves a task with its required checks;
             * Execution makes its worktree; the worker (the Assistant) changes the code;
             * the Studio runs every required check on the task commit and
             * Implementation judges them; the Operator reviews and integrates exactly
             * that commit, builds it and adopts the build, which the launcher starts.
             */
        subject product : Agentique;
        objective {
            verify requiredChecksDecide;
            verify recoverableAdoption;
        }
        part changesTheCode : Scenarios::StandIn {
            :>> target = product.assistant;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new WorkDone(files = 3);
        }
        send new TaskApproval(required = 5, passing = 5) via product.operator;
        then accept review : Review via product.operator;
        then assert constraint everyRequiredCheckPassed {
            review.passed
        }
        then send new IntegrateRequest(withoutVerification = false) via product.operator;
        then accept integrated : Integrated via product.operator;
        then send new BuildRequest(builds = true) via product.operator;
        then accept built : Built via product.operator;
        then assert constraint integratedAndBuilt {
            integrated.ok and built.ok
        }
        then send new AdoptRequest(starts = true) via product.operator;
        then accept running : Running via product.operator;
        then assert constraint theNewBuildRuns {
            running.build == Started::newBuild
        }
    }

    verification def ARequiredCheckThatDidNotRun {
        doc /* A failure path of the development task: one required check did not pass (it
             * failed, or did not run). The verification does not pass, and integration
             * is refused unless the Operator says otherwise, explicitly.
             */
        subject product : Agentique;
        objective {
            verify requiredChecksDecide;
        }
        part changesTheCode : Scenarios::StandIn {
            :>> target = product.assistant;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new WorkDone(files = 2);
        }
        send new TaskApproval(required = 5, passing = 4) via product.operator;
        then accept review : Review via product.operator;
        then assert constraint notPassed {
            not review.passed
        }
        then send new IntegrateRequest(withoutVerification = false) via product.operator;
        then accept status : StatusMessage via product.operator;
        then assert constraint refused {
            status.reason == Refusal::notVerified
        }
    }

    verification def ANewBuildThatDoesNotStart {
        doc /* A failure path of adoption: the new build does not start; the launcher
             * starts the last known good build.
             */
        subject product : Agentique;
        objective {
            verify recoverableAdoption;
        }
        send new AdoptRequest(starts = false) via product.operator;
        then accept running : Running via product.operator;
        then assert constraint theLastKnownGoodRuns {
            running.build == Started::lastKnownGood
        }
    }

    requirement def Purpose {
        doc /* Agentique's purpose, as ROADMAP §1.1 states it (C-55; the one authoritative statement, referred to here, not restated). W13.4 states its obligations as subrequirements of the requirement `purpose`; the purpose and its protections change only by the Operator's decision, never inside an objective. */
        subject product : Agentique;
    }

    requirement purpose : Purpose;

    item def Findings {
        doc /* What an exploration of a running build reproduced: findings whose replay from a fresh start failed the same way twice and that the lead judged defects (C-54, C-55). */
        attribute reproduced : Natural;
    }

    item def LeadAnswer {
        doc /* What the lead hands back to the cycle: a proposal, or a delegation of a child objective. */
    }

    item def Proposal :> LeadAnswer {
        doc /* One improvement with frozen criteria (C-53, C-55). servesRequirement: every requirement it names exists in the project's model; defectShownOnBase: one criterion fails on the original implementation with evidence; touchesPurpose: it changes ROADMAP.md or the purpose requirement. */
        attribute servesRequirement : Boolean;
        attribute defectShownOnBase : Boolean;
        attribute touchesPurpose : Boolean;
    }

    item def Delegation :> LeadAnswer {
        doc /* The lead asks for a child objective with its delegate tool, within the parent's permissions and budgets (C-54). */
        attribute focus : String;
    }

    item def ReviewVerdict {
        doc /* The independent reviewer's verdict on a change, against the frozen criteria, the check results and the cumulative change since the approved baseline (C-55). */
        attribute approved : Boolean;
    }

    port def ExplorationPort {
        doc /* The explorer's side: a directive with the objective's goal in, what it reproduced out. */
        in item directive : Directive;
        out item findings : Findings;
    }

    port def LeadPort {
        doc /* The lead's side: a directive with the brief in, its answer out. */
        in item directive : Directive;
        out item answer : LeadAnswer;
    }

    port def ImplementationPort {
        doc /* The implementer's side: the brief, or what to repair, in; the work done out. */
        in item brief : Brief;
        out item done : WorkDone;
    }

    port def ReviewPort {
        doc /* The reviewer's side: a directive with the change to review in, its verdict out. */
        in item directive : Directive;
        out item verdict : ReviewVerdict;
    }

    part def Explorer :> Agents::Agent {
        doc /* Operates a test instance of the running build through its control interface toward the objective's goal, choosing among the valid actions; invariants and stated expectations are checked after each action, and a finding is a check that failed and reproduced (C-54). The model of the explorer role in Settings answers it. */
        :>> mode = Agents::AgentMode::fast;
        :>> model = "deepseek-flash";
        port goal : ExplorationPort;
        port app : ~ControlPort;
    }

    part def Lead :> Agents::Agent {
        doc /* Inspects the model, the code, the findings and the running application, reading only, and proposes one improvement with frozen criteria, naming the requirements it serves, the elements it affects, its benefit and its effect on root complexity (C-53, C-55); or delegates a child objective. */
        :>> mode = Agents::AgentMode::deliberate;
        :>> model = "claude-opus-5-5";
        port work : LeadPort;
    }

    part def Implementer :> Agents::Agent {
        doc /* Implements the frozen proposal in the cycle's worktree with the development tools, changing the model only through Agentique's tools. */
        :>> mode = Agents::AgentMode::deliberate;
        :>> model = "claude-sonnet-5-5";
        port work : ImplementationPort;
    }

    part def Reviewer :> Agents::Agent {
        doc /* Reviews the change with fresh context and no write access, against the frozen criteria, the check results and the cumulative change since the approved baseline (C-55); its approval never overrides a failing check. */
        :>> mode = Agents::AgentMode::deliberate;
        :>> model = "claude-opus-5-5";
        port review : ReviewPort;
    }

    part def Cycle {
        doc /* The Orchestrator's deterministic part (C-53): it decides an objective's next phase from recorded results (explore and reproduce, propose, implement, check, review, merge, build, adopt, continue), hands work to the agents as directives, and never lets a judgment override a failing check or a gate. Repeated failure ends a cycle with a report; adoption starts the next one by itself while cycles are left. */
        port objectives : ObjectivePort;
        port explorer : ~ExplorationPort;
        port lead : ~LeadPort;
        port implementer : ~ImplementationPort;
        port reviewer : ~ReviewPort;
        port jobs : ~JobPort;
        port launch : ~LaunchPort;
        attribute depth : Natural = 0 {
            doc /* 0 for the Operator's objective, one more than its parent's for a child. */
        }
        attribute maxDepth : Natural = 2 {
            doc /* Children nest at most two deep (C-54). */
        }
        attribute cyclesLeft : Natural = 0 {
            doc /* The cycle budget left. */
        }
        attribute attempts : Natural = 0 {
            doc /* Attempts used in the current phase. */
        }
        attribute maxAttempts : Natural = 2 {
            doc /* The same failure twice in a row, or no progress, ends the cycle. */
        }
        attribute requiredChecks : Natural = 5 {
            doc /* The required checks a cycle's commit must pass, fixed at the proposal. */
        }
        attribute reproduced : Natural = 0 {
            doc /* Findings the last exploration reproduced. */
        }
        attribute touchesPurpose : Boolean = false {
            doc /* Whether the proposal changes ROADMAP.md or the purpose requirement (C-55). */
        }
        attribute merged : Boolean = false {
            doc /* Whether the last cycle merged. */
        }
        attribute adopted : Boolean = false {
            doc /* Whether the last cycle's build was adopted. */
        }
        exhibit state running {
            entry;
            then idle;
            state idle;
            state exploring;
            state proposing;
            state implementing;
            state checking;
            state reviewing;
            state merging;
            state building;
            state adopting;
            transition explore first idle accept g : ObjectiveRequest via objectives if g.explore do action {
                assign depth := g.depth;
                then assign cyclesLeft := g.cycles;
                then send new Directive(delegates = false, refused = false) via explorer;
            } then exploring;
            transition propose first idle accept g : ObjectiveRequest via objectives if not g.explore do action {
                assign depth := g.depth;
                then assign cyclesLeft := g.cycles;
                then assign attempts := 0;
                then send new Directive(delegates = false, refused = false) via lead;
            } then proposing;
            transition found first exploring accept f : Findings via explorer if f.reproduced > 0 do action {
                assign reproduced := f.reproduced;
                then assign attempts := 0;
                then send new Directive(delegates = false, refused = false) via lead;
            } then proposing;
            transition nothingNew first exploring accept f : Findings via explorer if f.reproduced == 0 do send new CycleReport(reproduced = 0, merged = false, adopted = false) via objectives then idle;
            transition delegated first proposing accept d : Delegation via lead if depth < maxDepth do action {
                send new Directive(delegates = true, refused = false) via objectives;
                then send new Directive(delegates = false, refused = false) via lead;
            } then proposing;
            transition delegationRefused first proposing accept d : Delegation via lead if depth >= maxDepth do action {
                send new Directive(delegates = true, refused = true) via objectives;
                then send new Directive(delegates = false, refused = false) via lead;
            } then proposing;
            transition proposed first proposing accept p : Proposal via lead if p.servesRequirement and p.defectShownOnBase do action {
                assign touchesPurpose := p.touchesPurpose;
                then assign attempts := 0;
                then send new Brief(task = "cycle") via implementer;
            } then implementing;
            transition askedAgain first proposing accept p : Proposal via lead if (not p.servesRequirement or not p.defectShownOnBase) and attempts + 1 < maxAttempts do action {
                assign attempts := attempts + 1;
                then send new Directive(delegates = false, refused = false) via lead;
            } then proposing;
            transition noEvidence first proposing accept p : Proposal via lead if (not p.servesRequirement or not p.defectShownOnBase) and attempts + 1 >= maxAttempts do send new CycleReport(reproduced = reproduced, merged = false, adopted = false) via objectives then idle;
            transition check first implementing accept w : WorkDone via implementer do send new CheckRequest(required = requiredChecks, passing = requiredChecks) via jobs then checking;
            transition review first checking accept r : CheckResults via jobs if r.passed == r.required do send new Directive(delegates = false, refused = false) via reviewer then reviewing;
            transition repair first checking accept r : CheckResults via jobs if r.passed < r.required and attempts + 1 < maxAttempts do action {
                assign attempts := attempts + 1;
                then send new Brief(task = "repair") via implementer;
            } then implementing;
            transition noProgress first checking accept r : CheckResults via jobs if r.passed < r.required and attempts + 1 >= maxAttempts do send new CycleReport(reproduced = reproduced, merged = false, adopted = false) via objectives then idle;
            transition approvedToMerge first reviewing accept v : ReviewVerdict via reviewer if v.approved and not touchesPurpose do send new Integrate(task = "cycle") via jobs then merging;
            transition purposeProtected first reviewing accept v : ReviewVerdict via reviewer if touchesPurpose do send new CycleReport(reproduced = reproduced, merged = false, adopted = false) via objectives then idle;
            transition changesRequested first reviewing accept v : ReviewVerdict via reviewer if not v.approved and not touchesPurpose and attempts + 1 < maxAttempts do action {
                assign attempts := attempts + 1;
                then send new Brief(task = "repair") via implementer;
            } then implementing;
            transition rejected first reviewing accept v : ReviewVerdict via reviewer if not v.approved and not touchesPurpose and attempts + 1 >= maxAttempts do send new CycleReport(reproduced = reproduced, merged = false, adopted = false) via objectives then idle;
            transition build first merging accept i : Integrated via jobs if i.ok do action {
                assign merged := true;
                then send new BuildRequest(builds = true) via jobs;
            } then building;
            transition adopt first building accept b : Built via jobs if b.ok do send new AdoptRequest(starts = true) via launch then adopting;
            transition notBuilt first building accept b : Built via jobs if not b.ok do send new CycleReport(reproduced = reproduced, merged = true, adopted = false) via objectives then idle;
            transition continues first adopting accept r : Running via launch if r.build == Started::newBuild and cyclesLeft > 1 do action {
                assign adopted := true;
                then assign cyclesLeft := cyclesLeft - 1;
                then send new CycleReport(reproduced = reproduced, merged = true, adopted = true) via objectives;
                then send new Directive(delegates = false, refused = false) via explorer;
            } then exploring;
            transition finished first adopting accept r : Running via launch if r.build == Started::newBuild and cyclesLeft <= 1 do action {
                assign adopted := true;
                then send new CycleReport(reproduced = reproduced, merged = true, adopted = true) via objectives;
            } then idle;
            transition recovered first adopting accept r : Running via launch if r.build == Started::lastKnownGood do send new CycleReport(reproduced = reproduced, merged = true, adopted = false) via objectives then idle;
        }
    }

    verification def AnObjectiveAdoptsAndExploresAgain {
        doc /* From one intent: the explorer reproduces a finding, the lead proposes a fix that names the requirement it serves and shows the defect on the base, the implementer makes it, the required checks pass, the reviewer approves, the change is merged, built and adopted, and the adopted build explores again by itself, this time finding nothing new (Scenario K, C-54, C-55). The agents' judgments are stood in; the cycle's decisions are the model's own. */
        subject orchestrator : Orchestrator;
        objective {
            verify gatesDecide;
            verify findingsReproduce;
        }
        part finds : Scenarios::StandIn {
            :>> call = 1;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new Findings(reproduced = 1);
            :>> target = orchestrator.explorer;
        }
        part findsNothingNew : Scenarios::StandIn {
            :>> call = 2;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new Findings(reproduced = 0);
            :>> target = orchestrator.explorer;
        }
        part proposes : Scenarios::StandIn {
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new Proposal(servesRequirement = true, defectShownOnBase = true, touchesPurpose = false);
            :>> target = orchestrator.lead;
        }
        part implements : Scenarios::StandIn {
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new WorkDone(files = 3);
            :>> target = orchestrator.implementer;
        }
        part approves : Scenarios::StandIn {
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new ReviewVerdict(approved = true);
            :>> target = orchestrator.reviewer;
        }
        send new ObjectiveRequest(cycles = 2, explore = true, depth = 0) via orchestrator.objectives;
        then accept checks : CheckRequest via orchestrator.jobs;
        then send new CheckResults(required = 5, passed = 5) via orchestrator.jobs;
        then accept 'merge' : Integrate via orchestrator.jobs;
        then send new Integrated(ok = true) via orchestrator.jobs;
        then accept build : BuildRequest via orchestrator.jobs;
        then send new Built(ok = true) via orchestrator.jobs;
        then accept adopt : AdoptRequest via orchestrator.launch;
        then send new Running(build = Started::newBuild) via orchestrator.launch;
        then accept adopted : CycleReport via orchestrator.objectives;
        then assert constraint mergedAndAdopted {
            adopted.merged and adopted.adopted and adopted.reproduced == 1
        }
        then accept again : CycleReport via orchestrator.objectives;
        then assert constraint exploredAgainByItself {
            again.reproduced == 0 and not again.merged and orchestrator.cycle.adopted
        }
    }

    verification def AProposalWithoutEvidenceIsRefused {
        doc /* A failure path: the lead's proposal shows no defect on the original implementation (no criterion fails there with evidence). It is asked again once; the same answer ends the cycle with a report, and nothing reaches the checks, the merge or the build (C-54, DefectShownBefore). */
        subject orchestrator : Orchestrator;
        objective {
            verify defectShownBefore;
        }
        part noEvidence : Scenarios::StandIn {
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new Proposal(servesRequirement = true, defectShownOnBase = false, touchesPurpose = false);
            :>> target = orchestrator.lead;
        }
        send new ObjectiveRequest(cycles = 1, explore = false, depth = 0) via orchestrator.objectives;
        then accept report : CycleReport via orchestrator.objectives;
        then assert constraint nothingMerged {
            not report.merged and not orchestrator.cycle.merged and orchestrator.cycle.attempts == 1
        }
    }

    verification def AFailingCheckIsRepaired {
        doc /* A failure path recovered: one required check fails on the cycle's commit; the implementer repairs it within the attempt budget, the checks pass, and the cycle is reviewed, merged and adopted (C-53, GatesDecide). */
        subject orchestrator : Orchestrator;
        objective {
            verify gatesDecide;
        }
        part proposes : Scenarios::StandIn {
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new Proposal(servesRequirement = true, defectShownOnBase = true, touchesPurpose = false);
            :>> target = orchestrator.lead;
        }
        part implements : Scenarios::StandIn {
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new WorkDone(files = 3);
            :>> target = orchestrator.implementer;
        }
        part approves : Scenarios::StandIn {
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new ReviewVerdict(approved = true);
            :>> target = orchestrator.reviewer;
        }
        send new ObjectiveRequest(cycles = 1, explore = false, depth = 0) via orchestrator.objectives;
        then accept firstChecks : CheckRequest via orchestrator.jobs;
        then send new CheckResults(required = 5, passed = 4) via orchestrator.jobs;
        then accept checks : CheckRequest via orchestrator.jobs;
        then send new CheckResults(required = 5, passed = 5) via orchestrator.jobs;
        then accept 'merge' : Integrate via orchestrator.jobs;
        then send new Integrated(ok = true) via orchestrator.jobs;
        then accept build : BuildRequest via orchestrator.jobs;
        then send new Built(ok = true) via orchestrator.jobs;
        then accept adopt : AdoptRequest via orchestrator.launch;
        then send new Running(build = Started::newBuild) via orchestrator.launch;
        then accept report : CycleReport via orchestrator.objectives;
        then assert constraint mergedAfterRepair {
            report.merged and report.adopted and orchestrator.cycle.attempts == 1
        }
    }

    verification def TheReviewerRejectsTwice {
        doc /* A failure path: the checks pass, but the independent reviewer asks for changes twice; the cycle ends with a report and is not merged, whatever the checks said (C-53, GatesDecide). */
        subject orchestrator : Orchestrator;
        objective {
            verify gatesDecide;
        }
        part proposes : Scenarios::StandIn {
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new Proposal(servesRequirement = true, defectShownOnBase = true, touchesPurpose = false);
            :>> target = orchestrator.lead;
        }
        part implements : Scenarios::StandIn {
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new WorkDone(files = 3);
            :>> target = orchestrator.implementer;
        }
        part rejects : Scenarios::StandIn {
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new ReviewVerdict(approved = false);
            :>> target = orchestrator.reviewer;
        }
        send new ObjectiveRequest(cycles = 1, explore = false, depth = 0) via orchestrator.objectives;
        then accept firstChecks : CheckRequest via orchestrator.jobs;
        then send new CheckResults(required = 5, passed = 5) via orchestrator.jobs;
        then accept checks : CheckRequest via orchestrator.jobs;
        then send new CheckResults(required = 5, passed = 5) via orchestrator.jobs;
        then accept report : CycleReport via orchestrator.objectives;
        then assert constraint notMerged {
            not report.merged and not orchestrator.cycle.merged
        }
    }

    verification def TheGoverningTextStaysTheOperators {
        doc /* A failure path: a proposal changes ROADMAP.md or the purpose requirement. Even with passing checks and the reviewer's approval, the gate refuses to merge it; such a change is the Operator's decision (C-55). */
        subject orchestrator : Orchestrator;
        objective {
            verify gatesDecide;
        }
        part changesThePurpose : Scenarios::StandIn {
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new Proposal(servesRequirement = true, defectShownOnBase = true, touchesPurpose = true);
            :>> target = orchestrator.lead;
        }
        part implements : Scenarios::StandIn {
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new WorkDone(files = 3);
            :>> target = orchestrator.implementer;
        }
        part approves : Scenarios::StandIn {
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new ReviewVerdict(approved = true);
            :>> target = orchestrator.reviewer;
        }
        send new ObjectiveRequest(cycles = 1, explore = false, depth = 0) via orchestrator.objectives;
        then accept checks : CheckRequest via orchestrator.jobs;
        then send new CheckResults(required = 5, passed = 5) via orchestrator.jobs;
        then accept report : CycleReport via orchestrator.objectives;
        then assert constraint refusedDespiteApproval {
            not report.merged and orchestrator.cycle.touchesPurpose
        }
    }

    verification def ANewBuildThatDoesNotStartIsRecovered {
        doc /* A failure path of adoption: the merged change's build does not start, so the launcher starts the last known good build, which records that the cycle merged but was not adopted (C-53, RecoverableAdoption). */
        subject orchestrator : Orchestrator;
        objective {
            verify recoverableAdoption;
        }
        part proposes : Scenarios::StandIn {
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new Proposal(servesRequirement = true, defectShownOnBase = true, touchesPurpose = false);
            :>> target = orchestrator.lead;
        }
        part implements : Scenarios::StandIn {
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new WorkDone(files = 3);
            :>> target = orchestrator.implementer;
        }
        part approves : Scenarios::StandIn {
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new ReviewVerdict(approved = true);
            :>> target = orchestrator.reviewer;
        }
        send new ObjectiveRequest(cycles = 1, explore = false, depth = 0) via orchestrator.objectives;
        then accept checks : CheckRequest via orchestrator.jobs;
        then send new CheckResults(required = 5, passed = 5) via orchestrator.jobs;
        then accept 'merge' : Integrate via orchestrator.jobs;
        then send new Integrated(ok = true) via orchestrator.jobs;
        then accept build : BuildRequest via orchestrator.jobs;
        then send new Built(ok = true) via orchestrator.jobs;
        then accept adopt : AdoptRequest via orchestrator.launch;
        then send new Running(build = Started::lastKnownGood) via orchestrator.launch;
        then accept report : CycleReport via orchestrator.objectives;
        then assert constraint mergedNotAdopted {
            report.merged and not report.adopted and not orchestrator.cycle.adopted
        }
    }

    verification def DelegationBeyondTheDepthIsRefused {
        doc /* A failure path of delegation: an objective already two levels deep asks for a child objective; the directive is recorded as refused and the lead must propose instead (C-54, ChildWorkBounded). */
        subject orchestrator : Orchestrator;
        objective {
            verify childWorkBounded;
        }
        part delegates : Scenarios::StandIn {
            :>> call = 1;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new Delegation(focus = "the Requirements panel");
            :>> target = orchestrator.lead;
        }
        part noEvidence : Scenarios::StandIn {
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new Proposal(servesRequirement = true, defectShownOnBase = false, touchesPurpose = false);
            :>> target = orchestrator.lead;
        }
        send new ObjectiveRequest(cycles = 1, explore = false, depth = 2) via orchestrator.objectives;
        then accept directive : Directive via orchestrator.objectives;
        then assert constraint recordedAsRefused {
            directive.delegates and directive.refused
        }
        then accept report : CycleReport via orchestrator.objectives;
        then assert constraint nothingMerged {
            not report.merged
        }
    }
}
