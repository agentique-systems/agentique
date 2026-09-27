package AgentiqueArchitecture {
    doc /* The logical architecture of Agentique (REALIGNMENT §3.6, R-14). Each part
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
        part 'agq-kernel' : Crate;
        part 'agq-kerml' : Crate;
        part 'agq-sysml' : Crate;
        part 'agq-kerml-syntax' : Crate;
        part 'agq-kerml-text' : Crate;
        part 'agq-kerml-semantics' : Crate;
        part 'agq-sysml-semantics' : Crate;
        part 'agq-standard-libraries' : Crate;
        part 'agq-runtime-publications' : Crate {
            doc /* Loads the precomputed standard library bundle. Retired in Stage 1
                 * when the language core no longer needs the bundle (§4.2). */
        }
        part 'agq-metamodel-gen' : Crate {
            doc /* Generates the KerML/SysML metamodel descriptors. */
        }
    }

    part def SystemState {
        doc /* The live state of one project: typed change operations, locks, undo,
             * change events and queries for views. Its operations are locked core (R-16). */
        part 'agq-modeling-workspace' : Crate;
        part 'agq-modeling-service' : Crate;
        part 'agq-modeling-view' : Crate {
            doc /* Queries for views. */
        }
    }

    part def History {
        doc /* Saves and loads projects as SysML text in git, with checkpoints, branches
             * and diffs. The persistence format is locked core (R-16). Until Stage 2
             * replaces it with git, History is the SQLite revision store below. */
        part 'agq-modeling-repository' : Crate;
        part 'agq-modeling-sqlite' : Crate;
    }

    part def Studio {
        doc /* The application shell the Operator works in: the Surface (spatial
             * rendering and direct manipulation), the Panels (inspector, requirements,
             * history, simulation, links) and the Conversation with the Assistant. */
        part 'agq-studio-native' : Crate;
        part 'agq-studio-platform' : Crate;
        part 'agq-studio-scene' : Crate;
    }

    part def Assistant {
        doc /* The Claude API client: tool-use loop, skills, and tools that call System
             * State operations. An Orchestrator comes later. */
        part 'agq-modeling-agent' : Crate;
    }

    part def Simulation {
        doc /* Stage 4: scenarios run over the architecture, with runs, traces and
             * separate check verdicts. No crate yet. */
    }

    part def ImplementationLinks {
        doc /* Stage 5: links from model elements to code and tests, and drift checks.
             * No crate yet. */
    }

    part def Agentique {
        doc /* The product and its parts. */
        part languageCore : LanguageCore;
        part systemState : SystemState;
        part history : History;
        part studio : Studio;
        part assistant : Assistant;
        part simulation : Simulation;
        part implementationLinks : ImplementationLinks;
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
    dependency from Assistant to SystemState;
    dependency from Assistant to LanguageCore {
        doc /* Element identities and model types in tool calls. */
    }
    dependency from SystemState to LanguageCore;
    dependency from SystemState to History;
    dependency from Simulation to LanguageCore;
    dependency from ImplementationLinks to SystemState;

    // Temporary dependencies of code that Stage 2 replaces with git-backed History.

    dependency from Studio to History {
        doc /* Temporary: agq-studio-native and agq-studio-platform open the SQLite
             * revision store directly instead of through the System State. */
    }
    dependency from Assistant to History {
        doc /* Temporary: agq-modeling-agent reads revisions from agq-modeling-repository. */
    }
    dependency from History to SystemState {
        doc /* Temporary: agq-modeling-repository stores agq-modeling-workspace
             * revision and project types. */
    }
    dependency from History to LanguageCore {
        doc /* Temporary: agq-modeling-repository uses agq-kernel and agq-kerml-text
             * directly. Git-backed History stores text and needs no language types. */
    }
}
