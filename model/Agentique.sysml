package AgentiqueArchitecture {
    doc /* Agentique: a workspace where the Operator understands and deliberately
         * changes a system through its architecture, while AI helps build and check
         * its implementation (ROADMAP §1.5). This model is Agentique's own
         * architecture, and the contract for its code (C-20): each part says what it
         * is for, what it owns, its contract and what it must not do; the crates
         * that implement it are its `Crate` parts; `dependency` lists every allowed
         * use of one part by another; the scenarios at the end walk the three
         * workflows of ROADMAP §2.8 C2 across the parts, with their failure paths.
         * Inside: one System State per project, its history in git, the Library,
         * scenarios and runs, implementation links and checks, controlled
         * execution, the Assistant, and the builds of Agentique itself. Outside:
         * a project's code and deployments, other people, and any sandbox the
         * host does not have. The scenarios run in model execution: they check
         * this model's account of the workflows, not the code; the code is checked
         * by the tests linked to each step (model/links.json). */

    private import ScalarValues::*;

    part def Crate {
        doc /* A Cargo package. A part usage typed by Crate and named after the package
             * says that the crate implements the enclosing part. */
    }

    // What the parts exchange.

    enum def Actor {
        doc /* Who made a change: the Operator, or the Assistant (its changes can be
             * undone together). */
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
             * reading changes nothing; applying makes one System State change. */
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
             * part. `locked` says whether it touches a locked element. */
        attribute locked : Boolean;
    }

    item def Change {
        doc /* One System State change: typed operations on elements by identity (create,
             * delete, rename, move, connect, set property, lock, unlock), by an actor,
             * against a base revision, with the locks the Operator confirmed. It
             * applies completely or not at all, and is one undo step. */
        attribute madeBy : Actor;
        attribute touchesLock : Boolean;
        attribute confirmed : Boolean;
    }

    item def Applied {
        doc /* The change event: the revision after the change, and what it created,
             * changed and deleted. */
        attribute revision : Natural;
    }

    item def Refused {
        doc /* A change was not applied, and why. */
        attribute reason : Refusal;
    }

    item def Documents {
        doc /* The printed text of the documents a change touched, and the identity
             * file, for History to save atomically. */
        attribute count : Natural;
    }

    item def Saved {
        doc /* Whether the save is durable (its journal written and the files in
             * place). */
        attribute ok : Boolean;
    }

    item def Load {
        doc /* Read the model folder: after a crash, the save journal is finished or
             * discarded first, so the model is the old one or the new one. */
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
             * now draw. */
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
             * before the Studio carries it out. */
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
             * changed (each change can be undone). */
        attribute changed : Boolean;
    }

    item def TaskApproval {
        doc /* The Operator approves a task: its outcome, affected parts, protected
             * boundaries, permitted writes and acceptance criteria, and its required
             * checks, fixed from now on. `passing` says, for the walkthrough, how many
             * of them the work will pass. */
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
             * the rules. */
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
             * else failed; it names the commit it checked. */
        attribute passed : Boolean;
    }

    item def Review {
        doc /* The task for the Operator to review: the model changes and the code
             * changes together, each required check with its outcome. */
        attribute passed : Boolean;
    }

    item def IntegrateRequest {
        doc /* The Operator integrates the reviewed task, or, after seeing what was not
             * verified, integrates it without a passing verification. */
        attribute withoutVerification : Boolean;
    }

    item def Integrate {
        doc /* Bring exactly the reviewed task commit into the project's branch. */
        attribute task : String;
    }

    item def Integrated {
        doc /* The branch is at the task commit; nothing else in the working copy
             * changed. */
        attribute ok : Boolean;
    }

    item def BuildRequest {
        doc /* Build Agentique from the integrated commit into a versioned folder.
             * `builds` says, for the walkthrough, whether it compiles. */
        attribute builds : Boolean;
    }

    item def Built {
        doc /* A build with its manifest: source commit, toolchain, checks, digests. */
        attribute ok : Boolean;
    }

    item def AdoptRequest {
        doc /* Use this build: the launcher starts it. `starts` says, for the
             * walkthrough, whether it starts and reports ready. */
        attribute starts : Boolean;
    }

    item def Running {
        doc /* Which build runs after adoption: the new one, or the last known good one
             * when the new one did not start. */
        attribute build : Started;
    }

    // The contracts: what goes in and out of each part's ports.

    port def OperatorPort {
        doc /* The Operator's side of the Studio: edits, undo and opening; messages to
             * the Assistant; tasks, builds and adoption; and what the Studio shows. */
        in item edit : Edit;
        in item undo : Undo;
        in item reopen : Reopen;
        in item answer : LockAnswer;
        in item intent : Intent;
        in item approval : TaskApproval;
        in item integrate : IntegrateRequest;
        in item build : BuildRequest;
        in item adopt : AdoptRequest;
        out item update : SurfaceUpdate;
        out item question : LockQuestion;
        out item status : StatusMessage;
        out item report : Report;
        out item review : Review;
        out item integrated : Integrated;
        out item built : Built;
        out item running : Running;
    }

    port def ChangePort {
        doc /* The System State's one change path: changes, undo and opening in; a
             * change event or a refusal out. */
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
             * calls and the reply out. */
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

    // The parts and the crates that implement them.

    part def LanguageCore {
        doc /* The KerML/SysML subset: one element tree, text parse and print, name
             * resolution, typing, specialisation, behaviour, expressions and
             * validation. Owns the meaning and validity of a model (it knows no UI,
             * storage or AI). Pure functions the System State, the Library and the
             * runs call. Depends on no other part. Locked core (R-16). */
        part 'agq-language' : Crate {
            doc /* The language core: the SysML subset as an element tree with parse,
                 * print and validate (Stage 1, R-3 rebuild). */
        }
    }

    part def SystemState {
        doc /* The live model of one project and its one change path. Owns the element
             * tree, the locks, the revision and the undo steps (in memory; saved
             * through History). Contract: a change applies completely or is refused
             * (stale, locked, invalid) and leaves everything as it was; a change that
             * cannot be saved is undone and said; a well-formed change that makes the
             * model invalid is applied and its problems shown (R-18). It does not
             * decide intent, and nothing else changes the model. Locked core (R-16). */
        part 'agq-system-state' : Crate;
        port changes : ChangePort;
        port storage : ~FolderPort;
        attribute revision : Natural = 0 {
            doc /* Counts changes since the project was opened; a change prepared against
                 * an older revision is stale. */
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
             * SysML. The persistence format is locked core (R-16). */
        part 'agq-history' : Crate {
            doc /* The model folder of a project: atomic saves of the text files and the
                 * identity and lock file, commits, branches and reading any version,
                 * through an embedded git (R-6). */
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
             * Meaning and validity stay in LanguageCore; applying stays in SystemState. */
        part 'agq-library' : Crate;
    }

    part def Studio {
        doc /* The application the Operator works in: the Surface, the Panels, the
             * Conversation, Settings and the task review. It carries out every edit and
             * every tool call through the System State's one change path (asking about
             * locks), runs tasks through Execution, and decides nothing the Operator
             * has not. Owns presentation (cameras, layouts, selection), the session,
             * Settings and each project's app data (conversation, jobs, runs,
             * required checks). */
        part 'agq-studio-native' : Crate;
        part 'agq-studio-scene' : Crate;
        port operator : OperatorPort;
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
            state assistantApplying;
            state assistantAsking;
            state preparing;
            state implementing;
            state checking;
            state judging;
            state reviewing;
            state integrating;
            state building;
            state adopting;
            // An ordinary edit.
            transition edit first idle accept e : Edit via operator do send new Change(madeBy = Actor::operator, touchesLock = e.locked, confirmed = false) via edits then applying;
            transition shown first applying accept a : Applied via edits do send new SurfaceUpdate(revision = a.revision) via operator then idle;
            transition askLock first applying accept r : Refused via edits if r.reason == Refusal::locked do send new LockQuestion(madeBy = Actor::operator) via operator then asking;
            transition notDone first applying accept r : Refused via edits if r.reason != Refusal::locked do send new StatusMessage(reason = r.reason) via operator then idle;
            transition confirmed first asking accept a : LockAnswer via operator if a.allow do send new Change(madeBy = Actor::operator, touchesLock = true, confirmed = true) via edits then applying;
            transition keptLocked first asking accept a : LockAnswer via operator if not a.allow do send new StatusMessage(reason = Refusal::locked) via operator then idle;
            transition undo first idle accept u : Undo via operator do send u via edits then applying;
            transition reopen first idle accept o : Reopen via operator do send o via edits then applying;
            // An Assistant action.
            transition ask first idle accept i : Intent via operator do send i via assistant then thinking;
            transition read first thinking accept c : ToolCall via assistant if c.tool == Tool::readModel do send new ToolResult(ok = true) via assistant then thinking;
            transition assistantChange first thinking accept c : ToolCall via assistant if c.tool == Tool::applyChanges do send new Change(madeBy = Actor::assistant, touchesLock = c.touchesLock, confirmed = false) via edits then assistantApplying;
            transition assistantApplied first assistantApplying accept a : Applied via edits do action {
                send new SurfaceUpdate(revision = a.revision) via operator;
                then send new ToolResult(ok = true) via assistant;
            } then thinking;
            transition assistantAskLock first assistantApplying accept r : Refused via edits if r.reason == Refusal::locked do send new LockQuestion(madeBy = Actor::assistant) via operator then assistantAsking;
            transition assistantRefused first assistantApplying accept r : Refused via edits if r.reason != Refusal::locked do send new ToolResult(ok = false) via assistant then thinking;
            transition assistantConfirmed first assistantAsking accept a : LockAnswer via operator if a.allow do send new Change(madeBy = Actor::assistant, touchesLock = true, confirmed = true) via edits then assistantApplying;
            transition assistantKeptLocked first assistantAsking accept a : LockAnswer via operator if not a.allow do send new ToolResult(ok = false) via assistant then thinking;
            transition reported first thinking accept r : Reply via assistant do send new Report(changed = r.changed) via operator then idle;
            // A development task.
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
    }

    part def Assistant {
        doc /* The AI agent the Operator works with (C-5): its turn runs on a runtime,
             * either Agentique's own loop over a provider or the Claude Agent runtime,
             * and every tool call it makes is checked against its schema and carried
             * out by the Studio on the one change path; in an implementation task it is
             * the worker, with the coding tools in the task's worktree. Owns the tool
             * definitions, the skills and the conversation's entries. Its behaviour
             * comes from a model, so the scenarios stand it in. It never changes the
             * model, the code or anything else except through those tools. */
        part 'agq-assistant' : Crate;
        port turn : TurnPort;
    }

    part def ClaudeAgentRuntime {
        doc /* The Claude Agent runtime (C-51, ROADMAP §4.7): a TypeScript companion
             * process (claude-agent/) that runs the official Claude Agent SDK's loop for
             * the Assistant, speaking protocol 1 with the Studio on standard input and
             * output. Owns the SDK's session files (in its own configuration folder).
             * Contract: the agent calls only Agentique's tools, through one MCP server
             * whose calls the Studio carries out; every built-in tool is off; nothing of
             * the machine's Claude Code configuration is loaded. Not a sandbox: it runs
             * with the Operator's rights and reaches Anthropic's API. No crate: its
             * code is in claude-agent/src. A safeguard: locked. */
    }

    part def Providers {
        doc /* Talks to model providers through rig (and a thin Jev client, C-34);
             * keys in the OS credential store; model lists, capabilities, usage. The
             * only part that uses rig, tokio, reqwest or the credential-store crates
             * (R-21, R-41). Owns the keys and the capability table. Temporary:
             * agq-assistant still uses reqwest for its hand-written Claude client until
             * W5.7 moves it here. */
        part 'agq-providers' : Crate {
            doc /* The provider layer: one streamed model call for every provider,
                 * the capability table, keys and usage (S4.2 kept, start of W5.7). */
        }
    }

    part def Simulation {
        doc /* Runs scenarios (C-50, ROADMAP §4.14): compiles a scenario and a fixed model
             * snapshot into disposable runtime structures and executes them with
             * stand-ins, recordings or a live model client it is given; records traces,
             * check verdicts and results with provenance, and works out freshness. Owns
             * run results (app data). It never changes the System State and never
             * reaches a provider or any other service by itself. */
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
             * A safeguard: locked. */
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
             * safeguard: locked. */
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
             * starts the last known good build, which says what happened. Owns the
             * builds registry. Depends on no other part, so it works when a new build
             * does not; it never builds, downloads or deletes. A safeguard: locked. */
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

    part def Agentique {
        doc /* The product and its parts, connected as they work together. The Operator
             * reaches it through the Studio. */
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
        connect operator to studio.operator;
        connect studio.edits to systemState.changes;
        connect systemState.storage to history.folder;
        connect studio.assistant to assistant.turn;
        connect studio.execution to execution.jobs;
        connect studio.verifying to implementation.verdict;
        connect studio.launcher to launcher.launch;
    }

    // Allowed dependencies, client to supplier. A crate may use crates of its own part
    // and of the parts its part depends on directly. Arrows point inward.

    dependency from Studio to SystemState;
    dependency from Studio to Assistant {
        doc /* The Conversation in the Studio sends the Operator's messages to the
             * Assistant and shows its replies (refines R-14). */
    }
    dependency from Studio to LanguageCore {
        doc /* Element identities and model types shown on the Surface. */
    }
    dependency from Studio to Providers {
        doc /* Settings tests keys and lists models. */
    }
    dependency from Studio to Library {
        doc /* The Library panel, insertion, previews and My Library. */
    }
    dependency from Assistant to SystemState;
    dependency from Assistant to LanguageCore {
        doc /* Element identities and model types in tool calls. */
    }
    dependency from Assistant to Providers {
        doc /* Model calls: requests, streamed replies, usage; and the Anthropic key for
             * the Claude Agent runtime's process. */
    }
    dependency from Assistant to Library {
        doc /* The Library tools: search, read and use building blocks. */
    }
    dependency from Assistant to ClaudeAgentRuntime {
        doc /* The Claude Agent runtime: started per turn, protocol 1 (C-51). */
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
             * them out (C-50). */
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
             * snapshot of the tree. */
    }
    dependency from Implementation to LanguageCore {
        doc /* Links name elements by identity; contract shapes come from item and enum
             * definitions. */
    }
    dependency from Implementation to Simulation {
        doc /* The implementation runner shares the run contract, the step interpreter
             * and the check evaluator with model execution. */
    }
    dependency from Implementation to Execution {
        doc /* Builds, tests and the harness run as controlled processes. */
    }

    // The guarantees the workflows keep, and the parts that keep them.

    requirement def OneChangePath {
        doc /* The Surface, the Inspector, the Library and the Assistant change the model
             * only through System State changes, each one undo step; a change to a
             * locked part applies only after the Operator confirms that change. */
        subject systemState : SystemState;
    }

    requirement def SavedBeforeShown {
        doc /* A change is durable before the Surface shows it as applied; one that cannot
             * be saved is undone and said; reopening shows the model as last saved. */
        subject systemState : SystemState;
    }

    requirement def AssistantThroughTools {
        doc /* The Assistant acts only through its tools, each checked and carried out by
             * the Studio on the one change path, whatever runtime runs its turn. */
        subject studio : Studio;
    }

    requirement def RequiredChecksDecide {
        doc /* A task's required checks are fixed when it is approved; it is integrated
             * only with a current, passing verification of all of them, or by the
             * Operator's explicit choice, recorded with the task. */
        subject implementation : Implementation;
    }

    requirement def RecoverableAdoption {
        doc /* A build that does not start leaves the last known good build running. */
        subject launcher : Launcher;
    }

    requirement oneChangePath : OneChangePath;
    requirement savedBeforeShown : SavedBeforeShown;
    requirement assistantThroughTools : AssistantThroughTools;
    requirement requiredChecksDecide : RequiredChecksDecide;
    requirement recoverableAdoption : RecoverableAdoption;

    part agentique : Agentique;

    satisfy oneChangePath by agentique.systemState;
    satisfy savedBeforeShown by agentique.systemState;
    satisfy assistantThroughTools by agentique.studio;
    satisfy requiredChecksDecide by agentique.implementation;
    satisfy recoverableAdoption by agentique.launcher;

    // The workflows (ROADMAP §2.8 C2), with their failure paths. Each runs in model
    // execution and steps across the parts; each step's code and tests are linked.

    verification def RenameAPart {
        doc /* An ordinary edit: the Operator renames a part; the System State applies the
             * change and History saves it before the Surface shows it; undo brings the
             * old name back, saved too; closing and reopening shows the model as last
             * saved. */
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
             * refused until the Operator confirms it; refused, nothing changes. */
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
             * bar says so. */
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
             * did. The Assistant's decisions come from a model, so they are stood in. */
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
             * Assistant is told, changes nothing and says so. */
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
             * that commit, builds it and adopts the build, which the launcher starts. */
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
             * is refused unless the Operator says otherwise, explicitly. */
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
             * starts the last known good build. */
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
}
