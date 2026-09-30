package AgentiqueArchitecture {
    doc /* The logical architecture of Agentique (ROADMAP §4.6, R-14). Each part
         * definition names the crates that implement it. The dependencies at the end
         * list every allowed use of one part by another; tools/check_architecture.py
         * compares both with the Cargo crate graph (R-15). */

    part def Crate {
        doc /* A Cargo package. A part usage typed by Crate and named after the package
             * says that the crate implements the enclosing part. */
    }

    // The parts and the crates that implement them.

    part def LanguageCore {
        doc /* The KerML/SysML subset: element graph, text parse and print, name
             * resolution, typing, specialisation and validation. Depends on no other
             * part. Locked core (R-16). */
        part 'agq-language' : Crate {
            doc /* The language core: the SysML subset as an element tree with parse,
                 * print and validate (Stage 1, R-3 rebuild). */
        }
    }

    part def SystemState {
        doc /* The live state of one project: typed change operations, locks, undo,
             * change events and queries for views. Its operations are locked core (R-16).
             * The Studio and the Assistant change the model only through atomic
             * changes of these operations: create, delete, rename, move, connect,
             * set property, lock and unlock. */
        part 'agq-system-state' : Crate;
    }

    part def History {
        doc /* Saves and loads projects as SysML text in git, with checkpoints, branches
             * and diffs. The persistence format is locked core (R-16). Stores text and
             * needs no language types. */
        part 'agq-history' : Crate {
            doc /* The model folder of a project: atomic saves of the text files and the
                 * identity and lock file, commits, branches and reading any version,
                 * through an embedded git (R-6). */
        }
    }

    part def Library {
        doc /* The Library of building blocks (C-49, ROADMAP §4.13): finds, describes and
             * searches reusable definitions (the built-in blocks, the project's own, My
             * Library), works out what a block needs, and plans its use, specialisation,
             * overrides and extraction as ordinary System State changes, the same for the
             * Studio and the Assistant. Meaning and validity stay in LanguageCore. */
        part 'agq-library' : Crate;
    }

    part def Studio {
        doc /* The application shell the Operator works in: the Surface (spatial
             * rendering and direct manipulation), the Panels (inspector, requirements,
             * history, simulation, links) and the Conversation with the Assistant. */
        part 'agq-studio-native' : Crate;
        part 'agq-studio-scene' : Crate;
    }

    part def Assistant {
        doc /* The AI agent the Operator works with: the turn loop, the tools that call
             * System State operations, the skills and the conversation. It talks to
             * models through Providers. Temporary: agq-assistant uses reqwest directly
             * for its hand-written Claude client until W5.7 moves it onto Providers.
             * An Orchestrator comes later. */
        part 'agq-assistant' : Crate;
    }

    part def Providers {
        doc /* Talks to model providers through rig (and a thin Jev client, C-34);
             * keys in the OS credential store; model lists, capabilities, usage. The
             * only part that uses rig, tokio, reqwest or the credential-store crates
             * (R-21, R-41). */
        part 'agq-providers' : Crate {
            doc /* The provider layer: one streamed model call for every provider,
                 * the capability table, keys and usage (S4.2 kept, start of W5.7). */
        }
    }

    part def Simulation {
        doc /* Runs scenarios (C-50, ROADMAP §4.14): compiles a scenario and a fixed model
             * snapshot into disposable runtime structures and executes them with
             * stand-ins, recordings or a live model client it is given; records traces,
             * check verdicts and results with provenance, and works out freshness. It
             * never changes the System State and never reaches a provider or any other
             * service by itself. */
        part 'agq-simulation' : Crate;
    }

    part def Implementation {
        doc /* What the model says about code (C-50, ROADMAP §4.15): implementation links,
             * code-to-model lookup, the implementation runner that drives real code
             * through the project's harness, and the supported checks (dependency
             * boundaries, mapped contract shapes, linked tests), whose failures are
             * drift. Its processes run through Execution. */
        part 'agq-implementation' : Crate;
    }

    part def Execution {
        doc /* Controlled side effects outside the System State (C-50, ROADMAP §4.15):
             * repository reads, scoped writes in a worktree, allow-listed commands with
             * scrubbed environments, worktrees and integration, as jobs with journals
             * that can be cancelled and resumed without repeating a completed side
             * effect. It reports the isolation the host really offers. */
        part 'agq-execution' : Crate;
    }

    part def Agentique {
        doc /* The product and its parts. */
        part languageCore : LanguageCore;
        part systemState : SystemState;
        part history : History;
        part 'library' : Library;
        part studio : Studio;
        part assistant : Assistant;
        part providers : Providers;
        part simulation : Simulation;
        part implementation : Implementation;
        part execution : Execution;
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
        doc /* Model calls: requests, streamed replies, usage. */
    }
    dependency from Assistant to Library {
        doc /* The Library tools: search, read and use building blocks. */
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
}
